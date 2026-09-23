//! Loopback HTTP that streams each open recording to its video editor. WebKitGTK's player can't
//! read a custom URI scheme (its GStreamer source takes only http(s) and blob URIs), and a blob
//! would hold the whole file in the web process. Each open video editor gets one file behind a
//! random token; the request path is only ever a lookup key, never a file path.

use crate::err;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

/// Token → (editor window label, canonical path).
static FILES: Mutex<BTreeMap<String, (String, PathBuf)>> = Mutex::new(BTreeMap::new());
/// The server's port once it's listening; a failed start is retried by the next editor.
static PORT: Mutex<Option<u16>> = Mutex::new(None);
/// Connections being served. A player opens a few at a time; more are dropped.
static CONNS: AtomicUsize = AtomicUsize::new(0);
const MAX_CONNS: usize = 32;
/// The whole request head must arrive within this (a slow client can't hold a thread).
const HEAD_BUDGET: Duration = Duration::from_secs(10);

/// The server's port; it starts on first use, once per process.
fn port() -> Result<u16, String> {
    let mut port = PORT.lock().unwrap();
    if let Some(p) = *port {
        return Ok(p);
    }
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(err)?;
    let p = listener.local_addr().map_err(err)?.port();
    std::thread::Builder::new()
        .name("rshot-stream".into())
        .spawn(move || accept(listener, p))
        .map_err(err)?;
    *port = Some(p);
    Ok(p)
}

fn accept(listener: TcpListener, port: u16) {
    for conn in listener.incoming() {
        let Ok(conn) = conn else {
            // e.g. out of file descriptors: back off rather than spin.
            std::thread::sleep(Duration::from_millis(50));
            continue;
        };
        if CONNS.fetch_add(1, Ordering::AcqRel) >= MAX_CONNS {
            CONNS.fetch_sub(1, Ordering::AcqRel);
            continue; // dropped: closed unanswered
        }
        let slot = Slot;
        // A thread that can't start drops the connection (and its slot) instead of panicking.
        let _ = std::thread::Builder::new().spawn(move || {
            let _slot = slot;
            serve(conn, port);
        });
    }
}

/// One of the MAX_CONNS connection slots, given back when the connection ends.
struct Slot;

impl Drop for Slot {
    fn drop(&mut self) {
        CONNS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Streams `canon` to editor window `label` until `revoke(label)`.
pub fn share(label: &str, canon: PathBuf) -> Result<(), String> {
    port()?;
    FILES.lock().unwrap().insert(token(), (label.into(), canon));
    Ok(())
}

pub fn revoke(label: &str) {
    FILES.lock().unwrap().retain(|_, (l, _)| l != label);
}

/// The URL of editor window `label`'s stream, if it has one.
pub fn url(label: &str) -> Option<String> {
    let port = (*PORT.lock().unwrap())?;
    FILES
        .lock()
        .unwrap()
        .iter()
        .find(|(_, (l, _))| l == label)
        .map(|(t, _)| format!("http://127.0.0.1:{port}/{t}"))
}

/// 128 random bits, as hex.
/// ponytail: std's SipHash, keyed from the OS RNG, over a counter; a CSPRNG crate if tokens ever
/// guard more than a loopback stream of a file the user already has.
fn token() -> String {
    use std::hash::{BuildHasher, Hasher};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let half = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(n);
        h.finish()
    };
    format!("{:016x}{:016x}", half(), half())
}

/// Reads that share one deadline, however the bytes trickle in.
struct Deadline<'a> {
    conn: &'a TcpStream,
    until: Instant,
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.conn.set_read_timeout(Some(left))?;
        (&mut &*self.conn).read(buf)
    }
}

/// One request per connection (`Connection: close`): a head of at most 8 KiB, read within
/// HEAD_BUDGET, then the answer.
fn serve(mut conn: TcpStream, port: u16) {
    let until = Instant::now() + HEAD_BUDGET;
    let head = Deadline { conn: &conn, until };
    let mut req = BufReader::new(head.take(8192));
    let mut line = String::new();
    let _ = req.read_line(&mut line);
    let mut words = line.split_whitespace();
    let (method, target) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let (mut host, mut range) = (None, None);
    loop {
        let mut h = String::new();
        if req.read_line(&mut h).unwrap_or(0) == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "host" => host = Some(v.trim().to_string()),
                "range" => range = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    drop(req);
    let _ = respond(
        &mut conn,
        port,
        method,
        target,
        host.as_deref(),
        range.as_deref(),
    );
}

fn respond(
    out: &mut impl Write,
    port: u16,
    method: &str,
    target: &str,
    host: Option<&str>,
    range: Option<&str>,
) -> std::io::Result<()> {
    // Another Host is a page elsewhere reaching us through DNS rebinding.
    if !host_ok(host, port) {
        return out.write_all(empty("403 Forbidden").as_bytes());
    }
    if method != "GET" && method != "HEAD" {
        return out.write_all(empty("405 Method Not Allowed").as_bytes());
    }
    let path = target
        .strip_prefix('/')
        .and_then(|t| FILES.lock().unwrap().get(t).map(|(_, p)| p.clone()));
    let Some(mut file) = path.and_then(|p| File::open(p).ok()) else {
        return out.write_all(empty("404 Not Found").as_bytes());
    };
    let len = file.metadata()?.len();
    let (head, span) = file_head(range, len);
    out.write_all(head.as_bytes())?;
    if let (Some((first, last)), "GET") = (span, method) {
        file.seek(SeekFrom::Start(first))?;
        std::io::copy(&mut file.take(last - first + 1), out)?;
    }
    Ok(())
}

fn host_ok(host: Option<&str>, port: u16) -> bool {
    host == Some(format!("127.0.0.1:{port}").as_str())
}

/// A `Range` header against a `len`-byte file: `Ok(None)` = the whole file (no header, or one this
/// server ignores: malformed, or several ranges), `Ok(Some((first, last)))` (inclusive), or
/// `Err(())` = not satisfiable (416).
fn byte_range(header: Option<&str>, len: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some((a, b)) = header
        .and_then(|h| h.trim().strip_prefix("bytes="))
        .filter(|spec| !spec.contains(','))
        .and_then(|spec| spec.split_once('-'))
    else {
        return Ok(None);
    };
    let (a, b) = (a.trim(), b.trim());
    let num = |s: &str| s.parse::<u64>().ok();
    let from = |first: u64, last: u64| {
        if first >= len {
            Err(())
        } else {
            Ok(Some((first, last.min(len - 1))))
        }
    };
    match (num(a), num(b)) {
        // `-n`: the last n bytes.
        (None, Some(n)) if a.is_empty() => match n.min(len) {
            0 => Err(()),
            n => Ok(Some((len - n, len - 1))),
        },
        // `first-`: to the end.
        (Some(first), None) if b.is_empty() => from(first, u64::MAX),
        (Some(first), Some(last)) if first <= last => from(first, last),
        _ => Ok(None),
    }
}

/// The head of the answer for a `len`-byte file, and the inclusive byte span to send after it.
fn file_head(range: Option<&str>, len: u64) -> (String, Option<(u64, u64)>) {
    let (status, span) = match byte_range(range, len) {
        Err(()) => {
            let fields = [
                format!("Content-Range: bytes */{len}"),
                "Content-Length: 0".into(),
            ];
            return (head("416 Range Not Satisfiable", &fields), None);
        }
        Ok(None) => ("200 OK", len.checked_sub(1).map(|last| (0, last))),
        Ok(Some(span)) => ("206 Partial Content", Some(span)),
    };
    let mut fields = vec![
        "Content-Type: video/mp4".to_string(),
        "Accept-Ranges: bytes".into(),
    ];
    if let (Some((first, last)), true) = (span, status.starts_with("206")) {
        fields.push(format!("Content-Range: bytes {first}-{last}/{len}"));
    }
    let n = span.map_or(0, |(first, last)| last - first + 1);
    fields.push(format!("Content-Length: {n}"));
    (head(status, &fields), span)
}

fn head(status: &str, fields: &[String]) -> String {
    let mut h = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for f in fields {
        h.push_str(f);
        h.push_str("\r\n");
    }
    h + "\r\n"
}

fn empty(status: &str) -> String {
    head(status, &["Content-Length: 0".into()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(method: &str, target: &str, host: &str) -> String {
        let mut out = Vec::new();
        respond(&mut out, 4321, method, target, Some(host), None).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn host_then_method_then_token_then_the_file() {
        let ours = "127.0.0.1:4321";
        assert!(answer("POST", "/nope", "evil.example:4321").starts_with("HTTP/1.1 403 "));
        assert!(answer("POST", "/nope", ours).starts_with("HTTP/1.1 405 "));
        assert!(answer("GET", "/nope", ours).starts_with("HTTP/1.1 404 "));
        assert!(answer("GET", "/../../etc/passwd", ours).starts_with("HTTP/1.1 404 "));
        let file = std::env::temp_dir().join(format!("rshot-stream-{}.mp4", std::process::id()));
        std::fs::write(&file, b"0123456789").unwrap();
        let token = "0123456789abcdef0123456789abcdef";
        FILES
            .lock()
            .unwrap()
            .insert(token.into(), ("respond-test".into(), file.clone()));
        let get = answer("GET", &format!("/{token}"), ours);
        let head = answer("HEAD", &format!("/{token}"), ours);
        let mut part = Vec::new();
        respond(
            &mut part,
            4321,
            "GET",
            &format!("/{token}"),
            Some(ours),
            Some("bytes=2-4"),
        )
        .unwrap();
        revoke("respond-test");
        let gone = answer("GET", &format!("/{token}"), ours);
        std::fs::remove_file(&file).unwrap();
        assert!(get.starts_with("HTTP/1.1 200 OK\r\n") && get.ends_with("\r\n\r\n0123456789"));
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n") && head.contains("Content-Length: 10\r\n"));
        assert!(head.ends_with("\r\n\r\n"), "HEAD sends no body: {head:?}");
        assert!(String::from_utf8_lossy(&part).ends_with("\r\n\r\n234"));
        assert!(gone.starts_with("HTTP/1.1 404 "));
    }

    #[test]
    fn only_our_own_host_is_served() {
        assert!(host_ok(Some("127.0.0.1:4321"), 4321));
        assert!(!host_ok(Some("127.0.0.1:4322"), 4321));
        assert!(!host_ok(Some("localhost:4321"), 4321));
        assert!(!host_ok(Some("evil.example:4321"), 4321));
        assert!(!host_ok(None, 4321));
    }

    #[test]
    fn single_byte_ranges() {
        assert_eq!(byte_range(None, 100), Ok(None));
        assert_eq!(byte_range(Some("bytes=0-9"), 100), Ok(Some((0, 9))));
        assert_eq!(byte_range(Some("bytes=90-200"), 100), Ok(Some((90, 99))));
        assert_eq!(byte_range(Some("bytes=10-"), 100), Ok(Some((10, 99))));
        assert_eq!(byte_range(Some("bytes=-30"), 100), Ok(Some((70, 99))));
        assert_eq!(byte_range(Some("bytes=-300"), 100), Ok(Some((0, 99))));
        // Not satisfiable.
        assert_eq!(byte_range(Some("bytes=100-"), 100), Err(()));
        assert_eq!(byte_range(Some("bytes=-0"), 100), Err(()));
        assert_eq!(byte_range(Some("bytes=0-"), 0), Err(()));
        // Ignored: the whole file.
        assert_eq!(byte_range(Some("bytes=9-3"), 100), Ok(None));
        assert_eq!(byte_range(Some("bytes=0-1,5-6"), 100), Ok(None));
        assert_eq!(byte_range(Some("items=0-9"), 100), Ok(None));
        assert_eq!(byte_range(Some("bytes=x-9"), 100), Ok(None));
    }

    #[test]
    fn heads_for_whole_partial_and_unsatisfiable() {
        const MP4: &str = "Content-Type: video/mp4\r\nAccept-Ranges: bytes\r\n";
        assert_eq!(
            file_head(None, 100),
            (
                format!("HTTP/1.1 200 OK\r\nConnection: close\r\n{MP4}Content-Length: 100\r\n\r\n"),
                Some((0, 99))
            )
        );
        assert_eq!(
            file_head(Some("bytes=10-19"), 100),
            (
                format!("HTTP/1.1 206 Partial Content\r\nConnection: close\r\n{MP4}Content-Range: bytes 10-19/100\r\nContent-Length: 10\r\n\r\n"),
                Some((10, 19))
            )
        );
        assert_eq!(
            file_head(Some("bytes=200-"), 100),
            (
                "HTTP/1.1 416 Range Not Satisfiable\r\nConnection: close\r\nContent-Range: bytes */100\r\nContent-Length: 0\r\n\r\n".into(),
                None
            )
        );
        assert_eq!(file_head(None, 0).1, None);
    }
}
