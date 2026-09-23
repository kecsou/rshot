//! Commands behind the image editor window. The window label maps to the file it edits.

use crate::{err, pipeline, store, thumbnail, ui, AppState};
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

/// (the path it was opened with, for the clipboard and display; its canonical path, for file I/O).
fn editor_paths(state: &AppState, window: &WebviewWindow) -> Result<(PathBuf, PathBuf), String> {
    state
        .editors
        .lock()
        .unwrap()
        .get(window.label())
        .cloned()
        .ok_or_else(|| "not an editor window".to_string())
}

/// async: building a window from a sync command deadlocks on Windows (WebView2).
#[tauri::command]
pub async fn open_editor(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let r = thumbnail::guard(&state, &path)
        .and_then(|(given, canon)| ui::open_editor(&app, &given, canon));
    // The thumbnail's click has no error UI: say it here (the card stays up to retry from).
    if let Err(e) = &r {
        pipeline::notify(&app, &format!("Couldn't open the editor: {e}"));
    }
    r
}

#[tauri::command]
pub fn editor_info(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<EditorInfo, String> {
    let (p, _) = editor_paths(&state, &window)?;
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
/// so the path the user already pasted keeps pointing at what they see. Ok(false) = written but
/// not copied: the file is kept and the editor offers a retry (spec §4).
/// async: writes up to ~16 MB and waits on the clipboard thread — keep it off the GTK main thread.
#[tauri::command]
pub async fn save_image(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<bool, String> {
    let InvokeBody::Raw(png) = request.body() else {
        return Err("expected PNG bytes".into());
    };
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("not a PNG".into());
    }
    let (opened, canon) = editor_paths(&state, &window)?;
    store::write_atomic(&canon, png).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    Ok(state
        .clipboard
        .copy_capture(&opened, Some(png), mode)
        .inspect_err(|e| eprintln!("rshot: saved, but not copied: {e}"))
        .is_ok())
}

/// Deletes the file and closes its editor (the Destroyed handler drops the map entry). A file
/// already gone (deleted from the thumbnail or a file manager) still closes it.
/// async: file I/O, and destroying the calling window, stay off the main thread.
#[tauri::command(async)]
pub fn editor_delete(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    match std::fs::remove_file(editor_paths(&state, &window)?.1) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(err(e)),
        _ => {}
    }
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
    let (given, _) = thumbnail::guard(&state, &path)?;
    state
        .clipboard
        .copy_capture(&given, None, store::ClipboardMode::PathOnly)
}
