//! What happens after pixels are chosen: PNG, atomic save, clipboard, sound, feedback.

use crate::{capture, store, store::Config, AppState};
use image::RgbaImage;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn notify(app: &AppHandle, msg: &str) {
    eprintln!("rshot: {msg}");
    let _ = app.notification().builder().title("rshot").body(msg).show();
}

/// Saves `img`, puts it on the clipboard and gives feedback. Returns the saved path.
pub fn finish_capture(app: &AppHandle, img: RgbaImage) -> Result<PathBuf, String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();
    let png = capture::encode_png(&img)?;
    let path = match store::save_screenshot(&cfg, &png) {
        Ok(p) => p,
        Err(e) => {
            // Never lose the capture: fall back to the image alone.
            let copied = state.clipboard.copy_image(&png).is_ok();
            return Err(format!(
                "Couldn't save to {}: {e}{}",
                store::screenshots_dir(&cfg).display(),
                if copied { " (image copied)" } else { "" }
            ));
        }
    };
    let copied = state
        .clipboard
        .copy_capture(&path, Some(&png), cfg.clipboard_mode);
    eprintln!("rshot: saved and copied at {}", crate::overlay::epoch_ms());
    finish(app, &cfg, &path, "image", copied);
    Ok(path)
}

/// After a recording: clipboard (path + file, never an image), then as for a screenshot.
pub fn finish_video(app: &AppHandle, path: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();
    let copied = state.clipboard.copy_capture(path, None, cfg.clipboard_mode);
    finish(app, &cfg, path, "video", copied);
    Ok(())
}

/// The shared end of a screenshot or a recording (`kind` "image"/"video"): last capture,
/// thumbnail card, shutter sound, and feedback when the clipboard failed.
fn finish(app: &AppHandle, cfg: &Config, path: &Path, kind: &str, copied: Result<(), String>) {
    let state = app.state::<AppState>();
    if let Err(e) = &copied {
        eprintln!("rshot: clipboard: {e}");
    }
    *state.last_capture.lock().unwrap() = Some(path.to_path_buf());
    *state.thumb.lock().unwrap() = Some(crate::thumbnail::Thumb {
        path: path.display().to_string(),
        display: crate::thumbnail::tildify(path),
        copied: copied.is_ok(),
        kind: kind.into(),
    });
    if cfg.shutter_sound {
        play_shutter(app);
    }
    if cfg.show_thumbnail {
        // The capture already succeeded; a missing thumbnail must not turn it into an error.
        if let Err(e) = crate::ui::show_thumbnail(app) {
            eprintln!("rshot: thumbnail: {e}");
        }
    } else {
        // An older card would otherwise linger with actions the guard now refuses.
        crate::ui::close_prefix(app, "thumbnail");
        if copied.is_err() {
            let what = if kind == "video" {
                "Recording"
            } else {
                "Screenshot"
            };
            notify(
                app,
                &format!("{what} saved, but copying to the clipboard failed"),
            );
        }
    }
}

pub fn capture_screen_now(app: &AppHandle) -> Result<(), String> {
    let hidden = crate::ui::clear_own_ui(app);
    let show_pointer = app.state::<AppState>().config.lock().unwrap().show_pointer;
    let frames = capture::grab_all(show_pointer)?;
    drop(hidden);
    let i = crate::ui::frame_under_cursor(app, &frames)?;
    let frame = frames.into_iter().nth(i).ok_or("no monitor found")?;
    finish_capture(app, frame.image).map(|_| ())
}

pub fn capture_window_now(app: &AppHandle) -> Result<(), String> {
    let hidden = crate::ui::clear_own_ui(app);
    let img = capture::focused_window_image();
    drop(hidden);
    finish_capture(app, img?).map(|_| ())
}

#[cfg(target_os = "linux")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    if let Ok(wav) = app
        .path()
        .resolve("sounds/shutter.wav", BaseDirectory::Resource)
    {
        // A thread waits on the player so no zombie process is left behind.
        std::thread::spawn(move || {
            if std::process::Command::new("pw-play")
                .arg(&wav)
                .status()
                .is_err()
            {
                let _ = std::process::Command::new("paplay").arg(&wav).status();
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    use windows::{
        core::HSTRING,
        Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME},
    };
    if let Ok(wav) = app
        .path()
        .resolve("sounds/shutter.wav", BaseDirectory::Resource)
    {
        // SAFETY: the path outlives the call; SND_ASYNC copies what it needs before returning.
        unsafe {
            let _ = PlaySoundW(
                &HSTRING::from(wav.as_os_str()),
                None,
                SND_FILENAME | SND_ASYNC,
            );
        }
    }
}

#[cfg(target_os = "macos")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    if let Ok(wav) = app
        .path()
        .resolve("sounds/shutter.wav", BaseDirectory::Resource)
    {
        // A thread waits on the player so no zombie process is left behind.
        std::thread::spawn(move || {
            let _ = std::process::Command::new("/usr/bin/afplay")
                .arg(wav)
                .status();
        });
    }
}
