//! What happens after pixels are chosen: PNG, atomic save, clipboard, sound, feedback.

use crate::{capture, err, store, AppState};
use image::RgbaImage;
use std::path::PathBuf;
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
    let saved = store::new_screenshot_path(&cfg).and_then(|p| store::write_atomic(&p, &png).map(|()| p));
    let path = match saved {
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
    let copied = state.clipboard.copy_capture(&path, Some(&png), cfg.clipboard_mode);
    *state.last_capture.lock().unwrap() = Some(path.clone());
    if cfg.shutter_sound {
        play_shutter(app);
    }
    if let Err(e) = copied {
        notify(app, &format!("Saved {}, but copying failed: {e}", path.display()));
    }
    Ok(path)
}

pub fn capture_screen_now(app: &AppHandle) -> Result<(), String> {
    let show_pointer = app.state::<AppState>().config.lock().unwrap().show_pointer;
    let mut frames = capture::grab_all(show_pointer)?;
    let pos = app.cursor_position().map_err(err)?;
    let i = capture::frame_at(&frames, pos.x as i32, pos.y as i32);
    finish_capture(app, frames.swap_remove(i).image).map(|_| ())
}

pub fn capture_window_now(app: &AppHandle) -> Result<(), String> {
    finish_capture(app, capture::focused_window_image()?).map(|_| ())
}

#[cfg(target_os = "linux")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    if let Ok(wav) = app.path().resolve("sounds/shutter.wav", BaseDirectory::Resource) {
        // A thread waits on the player so no zombie process is left behind.
        std::thread::spawn(move || {
            if std::process::Command::new("pw-play").arg(&wav).status().is_err() {
                let _ = std::process::Command::new("paplay").arg(&wav).status();
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn play_shutter(_app: &AppHandle) {} // ponytail: Windows/macOS sound arrives in Plan 4
