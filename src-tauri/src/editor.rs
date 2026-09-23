//! Commands behind the image editor window. The window label maps to the file it edits.

use crate::{err, store, thumbnail, ui, AppState};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    ipc::{InvokeBody, Request},
    AppHandle, Manager, State, WebviewWindow,
};

#[derive(Serialize)]
pub struct EditorInfo {
    path: String,
    display: String,
    name: String,
}

fn editor_path(state: &AppState, window: &WebviewWindow) -> Result<PathBuf, String> {
    state
        .editors
        .lock()
        .unwrap()
        .get(window.label())
        .map(|(opened, _)| opened.clone())
        .ok_or_else(|| "not an editor window".to_string())
}

/// async: building a window from a sync command deadlocks on Windows (WebView2).
#[tauri::command]
pub async fn open_editor(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let p = thumbnail::guard(&state, &path)?;
    ui::close_prefix(&app, "thumbnail");
    ui::open_editor(&app, &p)
}

#[tauri::command]
pub fn editor_info(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<EditorInfo, String> {
    let p = editor_path(&state, &window)?;
    Ok(EditorInfo {
        path: p.display().to_string(),
        display: thumbnail::tildify(&p),
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    })
}

/// Body = the flattened PNG. Overwrites the capture atomically and re-copies the clipboard,
/// so the path the user already pasted keeps pointing at what they see.
/// async: writes up to ~16 MB and waits on the clipboard thread — keep it off the GTK main thread.
#[tauri::command]
pub async fn save_image(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<(), String> {
    let InvokeBody::Raw(png) = request.body() else {
        return Err("expected PNG bytes".into());
    };
    let p = editor_path(&state, &window)?;
    store::write_atomic(&p, png).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&p, Some(png), mode)
}

/// Deletes the file and closes its editor (the Destroyed handler drops the map entry).
#[tauri::command]
pub fn editor_delete(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    std::fs::remove_file(editor_path(&state, &window)?).map_err(err)?;
    // Don't leave the tray / thumbnail pointing at a deleted file (Plan 1 final review).
    let forgot = {
        let mut last = state.last_capture.lock().unwrap();
        let gone = last.as_ref().is_some_and(|l| !l.exists());
        if gone {
            *last = None;
            *state.thumb.lock().unwrap() = None;
        }
        gone
    };
    if forgot {
        ui::close_prefix(window.app_handle(), "thumbnail");
    }
    window.destroy().map_err(err)
}

/// The path as text only (footer chip). Async like retry_copy: waits on the clipboard thread.
#[tauri::command(async)]
pub fn copy_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = thumbnail::guard(&state, &path)?;
    state
        .clipboard
        .copy_capture(&p, None, store::ClipboardMode::PathOnly)
}
