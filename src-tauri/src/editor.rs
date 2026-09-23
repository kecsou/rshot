//! Commands behind the image and video editor windows. The window label maps to the file it edits.

use crate::{err, pipeline, store, thumbnail, ui, AppState};
use serde::Serialize;
use std::path::{Path, PathBuf};
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

pub(crate) fn is_mp4(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
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
    // Image and video editors share a capability: each writes only its own kind of file.
    if !thumbnail::is_png(&canon) {
        return Err("not an image".into());
    }
    store::write_atomic(&canon, png).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    Ok(state
        .clipboard
        .copy_capture(&opened, Some(png), mode)
        .inspect_err(|e| eprintln!("rshot: saved, but not copied: {e}"))
        .is_ok())
}

/// Keeps [start, end] (seconds) of the open recording, muted or not: re-encodes it to a hidden
/// temp file next to it, then renames that over the original (spec §2.7), so the path the user
/// already pasted stays valid. Then re-copies it (path + file, never an image). Ok(false) = trimmed
/// but not copied, as for `save_image`: the page must not trim again, only offer a copy retry.
/// async: the re-encode takes seconds, on a blocking thread.
#[tauri::command]
pub async fn trim_video(
    window: WebviewWindow,
    state: State<'_, AppState>,
    start: f64,
    end: f64,
    mute: bool,
) -> Result<bool, String> {
    let (opened, canon) = editor_paths(&state, &window)?;
    if !is_mp4(&canon) {
        return Err("not a recording".into());
    }
    if !(start >= 0.0 && end > start) {
        return Err("invalid trim range".into());
    }
    let ffmpeg = crate::recorder::ffmpeg_path().ok_or(crate::recorder::NO_FFMPEG)?;
    let stem = canon
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = canon.with_file_name(format!(".{stem}.trim.mp4"));
    let args = crate::recorder::trim_args(&canon, start, end, mute, &tmp);
    let trimmed = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new(ffmpeg)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(crate::recorder::ffmpeg_log(true))
            .status()
    })
    .await
    .map_err(err)?
    .map_err(err)
    .and_then(|s| s.success().then_some(()).ok_or_else(|| s.to_string()))
    .and_then(|()| std::fs::rename(&tmp, &canon).map_err(err));
    if let Err(e) = trimmed {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Couldn't trim the video: {e}"));
    }
    let mode = state.config.lock().unwrap().clipboard_mode;
    Ok(state
        .clipboard
        .copy_capture(&opened, None, mode)
        .inspect_err(|e| eprintln!("rshot: trimmed, but not copied: {e}"))
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

#[cfg(test)]
mod tests {
    #[test]
    fn only_mp4s_are_trimmed() {
        use std::path::Path;
        assert!(super::is_mp4(Path::new("/v/Recording_1.mp4")));
        assert!(super::is_mp4(Path::new("/v/R.MP4")));
        assert!(!super::is_mp4(Path::new("/v/Screenshot_1.png")));
        assert!(!super::is_mp4(Path::new("/v/mp4")));
    }
}
