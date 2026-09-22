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

/// Only the last capture may be read, opened or deleted from a webview. Both paths are canonical.
/// (The screenshots folder is settable from a webview, so "anything in it" would be a hole.)
fn allowed(p: &Path, last: Option<&Path>) -> bool {
    last == Some(p)
}

fn guard(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path).canonicalize().map_err(err)?;
    let last = state
        .last_capture
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|l| l.canonicalize().ok());
    if allowed(&p, last.as_deref()) {
        Ok(p)
    } else {
        Err("not the last capture".into())
    }
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
pub fn open_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let p = guard(&state, &path)?;
    ui::close_prefix(&app, "thumbnail");
    app.opener()
        .open_path(p.to_string_lossy(), None::<&str>)
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
    Ok(())
}

#[tauri::command]
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

#[tauri::command]
pub fn dismiss_thumbnail(app: AppHandle) {
    ui::close_prefix(&app, "thumbnail");
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_last_capture_is_allowed() {
        let dir = std::env::temp_dir().join(format!("rshot-guard-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["last.png", "other.png"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let canon = |p: std::path::PathBuf| p.canonicalize().unwrap();
        let last = canon(dir.join("last.png"));
        assert!(super::allowed(
            &canon(dir.join("sub/../last.png")),
            Some(&last)
        ));
        assert!(!super::allowed(&canon(dir.join("other.png")), Some(&last)));
        assert!(!super::allowed(
            &canon(dir.join("sub/../other.png")),
            Some(&last)
        ));
        assert!(!super::allowed(&last, None));
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
