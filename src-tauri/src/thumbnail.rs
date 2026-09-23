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
/// so "anything in it" would be a hole.) Returns `path` as given, not canonical, so the clipboard
/// and the editor keep the path the user already has, even through a symlinked folder.
fn check(path: &str, last: Option<&Path>, open: &[PathBuf]) -> Result<PathBuf, String> {
    let p = Path::new(path).canonicalize().map_err(err)?;
    if last == Some(p.as_path()) || open.contains(&p) {
        Ok(PathBuf::from(path))
    } else {
        Err("not the last capture or a file open in the editor".into())
    }
}

pub(crate) fn guard(state: &AppState, path: &str) -> Result<PathBuf, String> {
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
    let p = guard(&state, &path)?;
    std::fs::read(p).map(Response::new).map_err(err)
}

#[tauri::command]
pub fn reveal_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(guard(&state, &path)?)
        .map_err(err)
}

#[tauri::command]
pub fn delete_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    std::fs::remove_file(guard(&state, &path)?).map_err(err)?;
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
    let p = guard(&state, &path)?;
    let png = std::fs::read(&p).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&p, Some(&png), mode)?;
    if let Some(t) = state.thumb.lock().unwrap().as_mut() {
        t.copied = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        // Through a symlinked folder the path comes back as given, not canonical.
        #[cfg(unix)]
        {
            let link = dir.with_extension("link");
            std::os::unix::fs::symlink(&dir, &link).unwrap();
            let given = link.join("edited.png");
            let got = super::check(given.to_str().unwrap(), None, &open);
            let other = super::check(link.join("other.png").to_str().unwrap(), None, &open);
            std::fs::remove_file(&link).unwrap();
            assert_eq!(got, Ok(given));
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
