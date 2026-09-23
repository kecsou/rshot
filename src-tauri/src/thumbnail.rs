//! Commands behind the floating thumbnail. Paths coming from the webview are checked first.

use crate::{err, ui, AppState};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{ipc::Response, AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize, Clone)]
pub struct Thumb {
    pub path: String,
    pub display: String,
    pub copied: bool,
    /// "image" or "video".
    pub kind: String,
}

/// `/home/me/Pictures/x.png` → `~/Pictures/x.png` (display only; the clipboard keeps the absolute path).
pub fn tildify(p: &Path) -> String {
    match dirs::home_dir().and_then(|h| p.strip_prefix(h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
    }
}

/// Only the last capture and files open in an editor may be read, opened or deleted from a
/// webview; `last` and `open` are canonical. (The screenshots folder is settable from a webview,
/// so "anything in it" would be a hole.) Returns `(given, canonical)`: file I/O must go through the
/// canonical path, the one that was checked (the given one may be re-pointed since). The given one
/// is only for the clipboard and display, so they keep the path the user already has, even
/// through a symlinked folder.
fn check(path: &str, last: Option<&Path>, open: &[PathBuf]) -> Result<(PathBuf, PathBuf), String> {
    let p = Path::new(path).canonicalize().map_err(err)?;
    if last == Some(p.as_path()) || open.contains(&p) {
        Ok((PathBuf::from(path), p))
    } else {
        Err("not the last capture or a file open in the editor".into())
    }
}

pub(crate) fn guard(state: &AppState, path: &str) -> Result<(PathBuf, PathBuf), String> {
    let last = state
        .last_capture
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|l| l.canonicalize().ok());
    // Editor paths were canonicalized when opened: compare as is, so a file swapped for a
    // symlink since then doesn't pass.
    let open: Vec<PathBuf> = state
        .editors
        .lock()
        .unwrap()
        .values()
        .map(|(_, canon)| canon.clone())
        .collect();
    check(path, last.as_deref(), &open)
}

#[tauri::command]
pub fn thumbnail_info(state: State<'_, AppState>) -> Option<Thumb> {
    state.thumb.lock().unwrap().clone()
}

#[tauri::command]
pub async fn read_capture(app: AppHandle, path: String) -> Result<Response, String> {
    let state = app.state::<AppState>();
    let (_, canon) = guard(&state, &path)?;
    // Pages draw PNGs from these bytes; a recording (possibly GBs) never crosses IPC.
    if !is_png(&canon) {
        return Err("not an image".into());
    }
    std::fs::read(canon).map(Response::new).map_err(err)
}

#[tauri::command]
pub fn reveal_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(guard(&state, &path)?.1)
        .map_err(err)
}

#[tauri::command]
pub fn delete_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    std::fs::remove_file(guard(&state, &path)?.1).map_err(err)?;
    ui::close_prefix(&app, "thumbnail");
    // Forget it (Open Last Capture, a new card) unless a newer capture already took its place.
    let mut last = state.last_capture.lock().unwrap();
    if last.as_ref().is_some_and(|l| !l.exists()) {
        *last = None;
        *state.thumb.lock().unwrap() = None;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn retry_copy(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let (given, canon) = guard(&state, &path)?;
    // A recording goes on as its path and file only, never as image/png.
    let png = is_png(&canon)
        .then(|| std::fs::read(&canon))
        .transpose()
        .map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&given, png.as_deref(), mode)?;
    if let Some(t) = state.thumb.lock().unwrap().as_mut() {
        t.copied = true;
    }
    Ok(())
}

pub(crate) fn is_png(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

pub(crate) fn is_mp4(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
}

/// A recording's duration and first frame, for the card, as raw bytes: the duration in seconds
/// (f64, little-endian; NaN if unknown), then the PNG, at most 460 px wide (the card's width ×2 for
/// HiDPI). From the bundled ffmpeg, so the card never plays the video and needs no GStreamer.
/// async: ffmpeg runs on a blocking thread, and is killed after 10 s.
#[tauri::command]
pub async fn video_poster(app: AppHandle, path: String) -> Result<Response, String> {
    let (_, canon) = guard(&app.state::<AppState>(), &path)?;
    if !is_mp4(&canon) {
        return Err("not a recording".into());
    }
    let ffmpeg = crate::recorder::ffmpeg_path().ok_or(crate::recorder::NO_FFMPEG)?;
    let mut cmd = crate::recorder::ffmpeg_command(&ffmpeg);
    cmd.args(crate::recorder::poster_args(&canon, 460));
    let out = tauri::async_runtime::spawn_blocking(move || {
        crate::recorder::output_within(cmd, std::time::Duration::from_secs(10))
    })
    .await
    .map_err(err)??;
    if !out.status.success() || out.stdout.is_empty() {
        return Err("ffmpeg couldn't read the recording".into());
    }
    let duration = crate::recorder::parse_duration(&String::from_utf8_lossy(&out.stderr));
    let mut body = duration.unwrap_or(f64::NAN).to_le_bytes().to_vec();
    body.extend_from_slice(&out.stdout);
    Ok(Response::new(body))
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_mp4s_are_recordings() {
        use std::path::Path;
        assert!(super::is_mp4(Path::new("/v/Recording_1.mp4")));
        assert!(super::is_mp4(Path::new("/v/R.MP4")));
        assert!(!super::is_mp4(Path::new("/v/Screenshot_1.png")));
        assert!(!super::is_mp4(Path::new("/v/mp4")));
    }

    #[test]
    fn only_pngs_go_on_the_clipboard_as_images() {
        use std::path::Path;
        assert!(super::is_png(Path::new("/v/Screenshot_1.png")));
        assert!(super::is_png(Path::new("/v/A.PNG")));
        assert!(!super::is_png(Path::new("/v/Recording_1.mp4")));
        assert!(!super::is_png(Path::new("/v/png")));
    }

    #[test]
    fn only_the_last_capture_or_an_open_file_is_allowed() {
        let dir = std::env::temp_dir().join(format!("rshot-guard-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["last.png", "other.png", "edited.png"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let canon = |p: std::path::PathBuf| p.canonicalize().unwrap();
        let last = canon(dir.join("last.png"));
        let open = [canon(dir.join("edited.png"))];
        let allowed = |p: &str, last: Option<&std::path::Path>, open: &[std::path::PathBuf]| {
            super::check(dir.join(p).to_str().unwrap(), last, open).is_ok()
        };
        assert!(allowed("sub/../last.png", Some(&last), &[]));
        assert!(!allowed("other.png", Some(&last), &[]));
        assert!(!allowed("sub/../other.png", Some(&last), &open));
        assert!(!allowed("last.png", None, &[]));
        // Still readable after another capture replaced "last" (or none is left).
        assert!(allowed("sub/../edited.png", Some(&last), &open));
        assert!(allowed("edited.png", None, &open));
        assert!(!allowed("edited.png", Some(&last), &[]));
        // Through a symlinked folder: the path as given (for the clipboard) plus the canonical
        // one that was checked (for file I/O).
        #[cfg(unix)]
        {
            let link = dir.with_extension("link");
            std::os::unix::fs::symlink(&dir, &link).unwrap();
            let given = link.join("edited.png");
            let got = super::check(given.to_str().unwrap(), None, &open);
            let other = super::check(link.join("other.png").to_str().unwrap(), None, &open);
            std::fs::remove_file(&link).unwrap();
            assert_eq!(got, Ok((given, open[0].clone())));
            assert!(other.is_err());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tildify_shortens_home_only() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            super::tildify(&home.join("Pictures/x.png")),
            "~/Pictures/x.png"
        );
        assert_eq!(
            super::tildify(std::path::Path::new("/tmp/x.png")),
            "/tmp/x.png"
        );
    }
}
