//! Settings window and first-run dialog.

use crate::{
    err, shortcuts,
    store::{self, ClipboardMode, Shortcuts},
    AppState,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub launch_at_login: bool,
    pub screenshots_dir: String,
    pub clipboard_mode: ClipboardMode,
    pub show_thumbnail: bool,
    pub shutter_sound: bool,
    pub takeover: bool,
    pub shortcuts: Shortcuts,
    #[serde(default)]
    pub takeover_error: Option<String>,
    #[serde(default)]
    pub manual: Vec<(String, String)>,
    pub recordings_dir: String,
    pub mic: Option<String>,
    pub fps: u8,
}

fn snapshot(app: &AppHandle, c: &store::Config, takeover_error: Option<String>) -> Settings {
    Settings {
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        screenshots_dir: store::screenshots_dir(c).display().to_string(),
        clipboard_mode: c.clipboard_mode,
        show_thumbnail: c.show_thumbnail,
        shutter_sound: c.shutter_sound,
        takeover: c.takeover,
        shortcuts: c.shortcuts.clone(),
        takeover_error,
        manual: shortcuts::manual_commands(c),
        recordings_dir: store::recordings_dir(c).display().to_string(),
        mic: c.mic.clone(),
        fps: c.fps,
    }
}

/// Takes the shortcuts over; on failure, puts back whatever was already changed.
fn take_over_or_roll_back(c: &mut store::Config) -> Result<(), String> {
    match shortcuts::take_over(c) {
        Ok(()) => {
            c.takeover = true;
            Ok(())
        }
        Err(e) => match shortcuts::restore(c) {
            Ok(()) => {
                c.takeover = false;
                Err(e)
            }
            // A kept backup means the system's keys may still be cleared: stay "on" so turning it off retries.
            Err(r) => {
                c.takeover = c.gnome_backup.is_some();
                Err(format!(
                    "{e}; putting the system's shortcuts back also failed: {r}"
                ))
            }
        },
    }
}

/// Called at startup: a takeover saved by an older rshot also takes the keys added since (the
/// record key), so a user who took over before them doesn't keep GNOME's recorder on it. On
/// Windows it arms the keyboard hook again (the bindings live in-process). A failure switches the
/// takeover off, as in Settings, and is returned.
pub fn catch_up_takeover(c: &mut store::Config) -> Result<(), String> {
    if !shortcuts::outdated(c) {
        return Ok(());
    }
    let taken = take_over_or_roll_back(c);
    let saved = store::save_config(c).map_err(err);
    taken.and(saved)
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Settings {
    snapshot(&app, &state.config.lock().unwrap(), None)
}

/// Async (off the main thread) here and below: up to ~13 gsettings runs.
#[tauri::command(async)]
pub fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Settings {
    let mut c = state.config.lock().unwrap();
    let mut error = None;
    let autostart = app.autolaunch();
    let was = autostart.is_enabled().unwrap_or(false);
    if settings.launch_at_login != was {
        let r = if settings.launch_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(e) = r {
            error = Some(format!("Launch at login: {e}"));
        }
    }
    c.screenshots_dir =
        store::dir_setting(&settings.screenshots_dir, &store::default_screenshots_dir());
    c.recordings_dir =
        store::dir_setting(&settings.recordings_dir, &store::default_recordings_dir());
    c.mic = settings.mic;
    c.fps = if settings.fps == 60 { 60 } else { 30 };
    c.clipboard_mode = settings.clipboard_mode;
    c.show_thumbnail = settings.show_thumbnail;
    c.shutter_sound = settings.shutter_sound;
    let rebound = c.shortcuts != settings.shortcuts;
    c.shortcuts = settings.shortcuts;
    if settings.takeover && (!c.takeover || rebound) {
        if let Err(e) = take_over_or_roll_back(&mut c) {
            error = Some(e);
        }
    } else if !settings.takeover && c.takeover {
        match shortcuts::restore(&mut c) {
            Ok(()) => c.takeover = false,
            Err(e) => error = Some(e),
        }
    }
    if let Err(e) = store::save_config(&c) {
        error = Some(format!("Saving settings: {e}"));
    }
    snapshot(&app, &c, error)
}

#[tauri::command(async)]
pub fn onboarding_choice(
    app: AppHandle,
    state: State<'_, AppState>,
    accept: bool,
) -> Result<(), String> {
    let mut c = state.config.lock().unwrap();
    c.onboarded = true;
    let result = if accept {
        let _ = app.autolaunch().enable();
        take_over_or_roll_back(&mut c)
    } else {
        Ok(())
    };
    store::save_config(&c).map_err(err)?;
    result
}

#[tauri::command]
pub fn open_config(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    store::save_config(&state.config.lock().unwrap()).map_err(err)?;
    app.opener()
        .open_path(store::config_path().to_string_lossy(), None::<&str>)
        .map_err(err)
}

/// While Settings records a new shortcut, rshot's own keys must reach it.
#[tauri::command]
pub fn set_rebinding(on: bool) {
    shortcuts::pause(on);
}

#[tauri::command]
pub fn close_window(window: WebviewWindow) {
    let _ = window.destroy();
}
