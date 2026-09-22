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
            // A kept backup means GNOME's keys may still be cleared: stay "on" so turning it off retries.
            Err(r) => {
                c.takeover = c.gnome_backup.is_some();
                Err(format!(
                    "{e}; putting GNOME's shortcuts back also failed: {r}"
                ))
            }
        },
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Settings {
    snapshot(&app, &state.config.lock().unwrap(), None)
}

#[tauri::command]
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
    c.screenshots_dir = store::dir_setting(&settings.screenshots_dir);
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

#[tauri::command]
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

#[tauri::command]
pub fn close_window(window: WebviewWindow) {
    let _ = window.destroy();
}
