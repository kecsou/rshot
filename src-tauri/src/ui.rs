//! Windows and tray.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

use crate::{capture::Frame, cli::Cmd, err};
use std::time::Duration;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &item("area", "Capture Area")?,
            &item("screen", "Capture Screen")?,
            &item("window", "Capture Window")?,
            &sep,
            &item("quit", "Quit rshot")?,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundle icon is configured"))
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            let cmd = match e.id.as_ref() {
                "quit" => return app.exit(0),
                "area" => Cmd::CaptureArea,
                "screen" => Cmd::CaptureScreen,
                "window" => Cmd::CaptureWindow,
                _ => return,
            };
            let app = app.clone();
            // Wait for the closing menu to leave the screen before grabbing it.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                crate::dispatch(&app, cmd);
            });
        })
        .build(app)?;
    Ok(())
}

pub fn overlay_index(label: &str) -> Option<usize> {
    label.strip_prefix("overlay-")?.parse().ok()
}

fn overlay_window(app: &AppHandle, i: usize) -> tauri::Result<WebviewWindow> {
    let label = format!("overlay-{i}");
    if let Some(w) = app.get_webview_window(&label) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("overlay/index.html".into()))
        .title("rshot overlay")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()
}

/// One hidden overlay per monitor, loaded ahead of time so Print feels instant.
pub fn ensure_overlays(app: &AppHandle) -> tauri::Result<()> {
    let n = xcap::Monitor::all().map(|m| m.len()).unwrap_or(1).max(1);
    for i in 0..n {
        overlay_window(app, i)?;
    }
    Ok(())
}

/// Puts overlay i fullscreen on frame i's monitor (only when it isn't there already).
pub fn place_overlays(app: &AppHandle, frames: &[Frame]) -> Result<(), String> {
    for (i, f) in frames.iter().enumerate() {
        let w = overlay_window(app, i).map_err(err)?;
        let pos = PhysicalPosition::new(f.x, f.y);
        if w.outer_position().ok() != Some(pos) || !w.is_fullscreen().unwrap_or(false) {
            w.set_fullscreen(false).map_err(err)?;
            w.set_position(pos).map_err(err)?;
            w.set_size(PhysicalSize::new(f.image.width(), f.image.height())).map_err(err)?;
            w.set_fullscreen(true).map_err(err)?;
        }
    }
    Ok(())
}

pub fn hide_overlays(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = w.hide();
        }
    }
    let _ = app.emit("overlay:hide", ());
}

/// Presents a window with a fresh X server timestamp so Mutter grants it focus. A re-shown
/// preloaded overlay otherwise carries its last user time (the previous Esc), which is older than
/// the window focused since, so focus-stealing prevention keeps focus away from it.
#[cfg(target_os = "linux")]
pub fn force_focus(w: &WebviewWindow) {
    let w2 = w.clone();
    let _ = w.run_on_main_thread(move || {
        use gtk::prelude::*;
        if let Ok(gw) = w2.gtk_window() {
            if let Some(gdk) = gw.window() {
                if let Ok(x11) = gdk.downcast::<gdkx11::X11Window>() {
                    let t = gdkx11::functions::x11_get_server_time(&x11);
                    x11.set_user_time(t);
                    gw.present_with_time(t);
                }
            }
        }
    });
}
