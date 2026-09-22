//! Clipboard: one thread owns the OS clipboard for the life of the daemon (on X11, pastes are
//! served by the owning process, so the owner must outlive the capture).

use crate::store::ClipboardMode;
use clipboard_rs::{Clipboard as _, ClipboardContent, ClipboardContext};
use std::{path::Path, sync::mpsc, thread};

type Job = (Vec<ClipboardContent>, mpsc::Sender<Result<(), String>>);

pub struct Clipboard {
    tx: mpsc::Sender<Job>,
}

impl Clipboard {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        thread::spawn(move || {
            let ctx = ClipboardContext::new();
            for (contents, reply) in rx {
                let result = match &ctx {
                    Ok(c) => c.set(contents).map_err(|e| e.to_string()),
                    Err(e) => Err(format!("clipboard unavailable: {e}")),
                };
                let _ = reply.send(result);
            }
        });
        Self { tx }
    }

    /// PathAndImage: path text + file (+ PNG when given). PathOnly: the path text alone (spec §2.4).
    pub fn copy_capture(
        &self,
        path: &Path,
        png: Option<&[u8]>,
        mode: ClipboardMode,
    ) -> Result<(), String> {
        match mode {
            ClipboardMode::PathAndImage => self.set(contents(path, png)),
            ClipboardMode::PathOnly => self.set(text_only(path)),
        }
    }

    /// Image only — used when saving failed, so the capture isn't lost.
    pub fn copy_image(&self, png: &[u8]) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        if !fits_x11(png) {
            return Err("image too large for the X11 clipboard".into());
        }
        self.set(image_only(png))
    }

    fn set(&self, contents: Vec<ClipboardContent>) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        self.tx.send((contents, reply)).map_err(|e| e.to_string())?;
        answer.recv().map_err(|e| e.to_string())?
    }
}

/// RFC 8089 file URI with percent-encoding (folders may be localised, e.g. "Vidéos").
#[cfg(target_os = "linux")]
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut s = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

// ponytail: 15 MiB image/png ceiling, since clipboard-rs lacks X11 INCR and a bigger property kills
// its serving thread; add INCR or a chunked writer if huge captures must paste as images.
#[cfg(target_os = "linux")]
const MAX_X11_PNG: usize = 15 * 1024 * 1024;

#[cfg(target_os = "linux")]
fn fits_x11(png: &[u8]) -> bool {
    png.len() <= MAX_X11_PNG
}

/// X11 targets rshot serves, in order.
#[cfg(target_os = "linux")]
pub fn linux_formats(path: &Path, png: Option<&[u8]>) -> Vec<(&'static str, Vec<u8>)> {
    let text = path.to_string_lossy().into_owned().into_bytes();
    let uri = file_uri(path);
    let mut v = vec![
        ("UTF8_STRING", text.clone()),
        ("text/plain;charset=utf-8", text.clone()),
        ("text/plain", text),
        ("text/uri-list", format!("{uri}\r\n").into_bytes()),
        (
            "x-special/gnome-copied-files",
            format!("copy\n{uri}").into_bytes(),
        ),
    ];
    if let Some(p) = png.filter(|p| fits_x11(p)) {
        v.push(("image/png", p.to_vec()));
    }
    v
}

#[cfg(target_os = "linux")]
fn contents(path: &Path, png: Option<&[u8]>) -> Vec<ClipboardContent> {
    linux_formats(path, png)
        .into_iter()
        .map(|(format, bytes)| match format {
            "UTF8_STRING" => ClipboardContent::Text(String::from_utf8_lossy(&bytes).into_owned()),
            _ => ClipboardContent::Other(format.to_string(), bytes),
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn image_only(png: &[u8]) -> Vec<ClipboardContent> {
    vec![ClipboardContent::Other("image/png".into(), png.to_vec())]
}

/// Path as text only: the three text targets, no file or image targets.
#[cfg(target_os = "linux")]
fn text_only(path: &Path) -> Vec<ClipboardContent> {
    contents(path, None).into_iter().take(3).collect()
}

#[cfg(not(target_os = "linux"))]
fn contents(path: &Path, png: Option<&[u8]>) -> Vec<ClipboardContent> {
    let p = path.to_string_lossy().into_owned();
    let mut v = vec![
        ClipboardContent::Text(p.clone()),
        ClipboardContent::Files(vec![p]),
    ];
    v.extend(image_only(png.unwrap_or_default()));
    v
}

#[cfg(not(target_os = "linux"))]
fn text_only(path: &Path) -> Vec<ClipboardContent> {
    vec![ClipboardContent::Text(path.to_string_lossy().into_owned())]
}

#[cfg(not(target_os = "linux"))]
fn image_only(png: &[u8]) -> Vec<ClipboardContent> {
    use clipboard_rs::{common::RustImage, RustImageData};
    RustImageData::from_bytes(png)
        .map(ClipboardContent::Image)
        .into_iter()
        .collect()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn file_uri_percent_encodes() {
        assert_eq!(
            file_uri(Path::new("/home/k/Vidéos/a b.png")),
            "file:///home/k/Vid%C3%A9os/a%20b.png"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/Screenshot_2026-09-22_20-41-07.png")),
            "file:///tmp/Screenshot_2026-09-22_20-41-07.png"
        );
    }

    #[test]
    fn formats_carry_path_uri_and_optional_png() {
        let p = Path::new("/tmp/s.png");
        let f = linux_formats(p, Some(b"PNG"));
        let get = |k: &str| {
            f.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, b)| b.clone())
                .unwrap()
        };
        assert_eq!(get("UTF8_STRING"), b"/tmp/s.png");
        assert_eq!(get("text/plain;charset=utf-8"), b"/tmp/s.png");
        assert_eq!(get("text/plain"), b"/tmp/s.png");
        assert_eq!(get("text/uri-list"), b"file:///tmp/s.png\r\n");
        assert_eq!(
            get("x-special/gnome-copied-files"),
            b"copy\nfile:///tmp/s.png"
        );
        assert_eq!(get("image/png"), b"PNG");
        assert!(linux_formats(p, None)
            .iter()
            .all(|(n, _)| *n != "image/png"));
    }

    #[test]
    fn oversized_png_is_dropped_but_path_targets_remain() {
        let f = linux_formats(Path::new("/tmp/s.png"), Some(&vec![0u8; MAX_X11_PNG + 1]));
        let names: Vec<&str> = f.iter().map(|(n, _)| *n).collect();
        assert!(!names.contains(&"image/png"));
        assert!(names.contains(&"UTF8_STRING") && names.contains(&"text/uri-list"));
    }

    #[test]
    fn x11_png_ceiling_is_inclusive() {
        assert!(fits_x11(&vec![0u8; MAX_X11_PNG]));
        assert!(!fits_x11(&vec![0u8; MAX_X11_PNG + 1]));
    }

    #[test]
    fn text_only_serves_just_the_path_text() {
        let formats: Vec<String> = text_only(Path::new("/tmp/s.png"))
            .into_iter()
            .map(|c| match c {
                ClipboardContent::Text(_) => "UTF8_STRING".to_string(),
                ClipboardContent::Other(f, _) => f,
                _ => "unexpected".to_string(),
            })
            .collect();
        assert_eq!(
            formats,
            ["UTF8_STRING", "text/plain;charset=utf-8", "text/plain"]
        );
    }
}
