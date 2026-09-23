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
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

/// Token → (editor window label, canonical path).
static FILES: Mutex<BTreeMap<String, (String, PathBuf)>> = Mutex::new(BTreeMap::new());
static PORT: OnceLock<Result<u16, String>> = OnceLock::new();

/// The server's port; it starts on first use, once per process.
fn port() -> Result<u16, String> {
    PORT.get_or_init(|| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(err)?;
        let port = listener.local_addr().map_err(err)?.port();
        std::thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                std::thread::spawn(move || serve(conn, port));
            }
        });
        Ok(port)
    })
    .clone()
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
    let port = *PORT.get()?.as_ref().ok()?;
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

/// One request per connection (`Connection: close`): a head of at most 8 KiB, then the answer.
fn serve(conn: TcpStream, port: u16) {
    let _ = conn.set_read_timeout(Some(Duration::from_secs(10)));
    let Ok(mut out) = conn.try_clone() else {
        return;
    };
    let mut req = BufReader::new(conn.take(8192));
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
    let _ = respond(
        &mut out,
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
