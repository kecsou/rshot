//! Windows and tray.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

use crate::{capture::Frame, cli::Cmd, err};
use std::time::Duration;
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

#[cfg(target_os = "linux")]
use gtk::prelude::*;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let sep = || tauri::menu::PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[
            &item("area", "Capture Area")?,
            &item("screen", "Capture Screen")?,
            &item("window", "Capture Window")?,
            &sep()?,
            &item("last", "Open Last Capture")?,
            &item("folder", "Open Screenshots Folder")?,
            &sep()?,
            &item("settings", "Settings…")?,
            &item("quit", "Quit rshot")?,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .cloned()
                .expect("bundle icon is configured"),
        )
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            let result = match e.id.as_ref() {
                "quit" => return app.exit(0),
                "settings" => open_settings(app),
                "last" => open_last(app),
                "folder" => open_folder(app),
                id => {
                    let cmd = match id {
                        "area" => Cmd::CaptureArea,
                        "screen" => Cmd::CaptureScreen,
                        "window" => Cmd::CaptureWindow,
                        _ => return,
                    };
                    let app = app.clone();
                    // Let the tray menu close so it isn't in the capture.
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(300));
                        crate::dispatch(&app, cmd);
                    });
                    Ok(())
                }
            };
            if let Err(e) = result {
                crate::pipeline::notify(app, &e);
            }
        })
        .build(app)?;
    Ok(())
}

fn open_last(app: &AppHandle) -> Result<(), String> {
    let last = app
        .state::<crate::AppState>()
        .last_capture
        .lock()
        .unwrap()
        .clone();
    let path = last.ok_or("No capture yet")?;
    if !path.exists() {
        return Err("The last capture no longer exists".into());
    }
    open_editor(app, &path)
}

fn open_folder(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::store::screenshots_dir(&app.state::<crate::AppState>().config.lock().unwrap());
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(err)
}

fn dialog_window(
    app: &AppHandle,
    label: &str,
    page: &str,
    title: &str,
    w: f64,
    h: f64,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(label) {
        win.show().map_err(err)?;
        // A plain set_focus loses to Mutter's focus-stealing prevention (window stays buried).
        #[cfg(target_os = "linux")]
        force_focus(&win);
        #[cfg(not(target_os = "linux"))]
        win.set_focus().map_err(err)?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .inner_size(w, h)
        .center()
        .build()
        .map(|_| ())
        .map_err(err)
}

pub fn open_settings(app: &AppHandle) -> Result<(), String> {
    dialog_window(
        app,
        "settings",
        "settings/index.html",
        "rshot Settings",
        720.0,
        680.0,
    )
}

pub fn open_onboarding(app: &AppHandle) -> Result<(), String> {
    dialog_window(
        app,
        "onboarding",
        "onboarding/index.html",
        "Welcome to rshot",
        480.0,
        400.0,
    )
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
            w.set_size(PhysicalSize::new(f.image.width(), f.image.height()))
                .map_err(err)?;
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
        if let Ok(gw) = w2.gtk_window() {
            present_now(gw.upcast_ref());
        }
    });
}

#[cfg(target_os = "linux")]
fn present_now(gw: &gtk::Window) {
    // A preloaded overlay has no GdkWindow until its first show, which may not have run yet:
    // without one this was a no-op, and the first overlay after any rshot dialog didn't get focus.
    gw.realize();
    if let Some(gdk) = gw.window() {
        if let Ok(x11) = gdk.downcast::<gdkx11::X11Window>() {
            let t = gdkx11::functions::x11_get_server_time(&x11);
            x11.set_user_time(t);
            gw.present_with_time(t);
        }
    }
}

/// rfd's GTK3 file dialogs ignore set_parent, and Mutter won't focus a new window that isn't a
/// transient of the focused one while an always-on-top overlay is up. So once the dialog shows,
/// make it the overlay's transient and focus it.
#[cfg(target_os = "linux")]
pub fn adopt_file_dialog(parent: &WebviewWindow) {
    let p = parent.clone();
    let _ = parent.run_on_main_thread(move || {
        let Ok(pw) = p.gtk_window() else { return };
        let mut tries = 0;
        gtk::glib::timeout_add_local(Duration::from_millis(50), move || {
            tries += 1;
            let dialog = gtk::Window::list_toplevels()
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::FileChooserDialog>().ok())
                .find(|d| d.is_visible());
            if let Some(d) = &dialog {
                d.set_transient_for(Some(&pw));
                present_now(d.upcast_ref());
            }
            if dialog.is_some() || tries >= 60 {
                gtk::glib::ControlFlow::Break
            } else {
                gtk::glib::ControlFlow::Continue
            }
        });
    });
}

static POPUP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Destroys every `{prefix}-*` window; true if there was one.
pub fn close_prefix(app: &AppHandle, prefix: &str) -> bool {
    let mut found = false;
    for (label, w) in app.webview_windows() {
        if label.starts_with(&format!("{prefix}-")) {
            found = true;
            let _ = w.destroy();
        }
    }
    found
}

/// Takes rshot's own UI off the screen before a new grab, so it can't end up in the capture: an
/// open overlay session, a countdown (dropping its pending capture) and the thumbnail card, all
/// always-on-top. Waits for the compositor only when something was showing. Not on the main thread.
pub fn clear_own_ui(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    let overlay = state.session.lock().unwrap().take().is_some();
    if overlay {
        hide_overlays(app);
    }
    state.pending.lock().unwrap().take();
    let countdown = close_prefix(app, "countdown");
    let thumbnail = close_prefix(app, "thumbnail");
    if overlay || countdown || thumbnail {
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// A small undecorated, transparent, always-on-top window (thumbnail, countdown).
pub fn popup(
    app: &AppHandle,
    prefix: &str,
    page: &str,
    w: f64,
    h: f64,
    focused: bool,
) -> Result<WebviewWindow, String> {
    close_prefix(app, prefix);
    let n = POPUP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    WebviewWindowBuilder::new(app, format!("{prefix}-{n}"), WebviewUrl::App(page.into()))
        .title("rshot")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(focused)
        .visible(false)
        .inner_size(w, h)
        .build()
        .map_err(err)
}

fn monitor_at(app: &AppHandle, x: f64, y: f64) -> Result<tauri::Monitor, String> {
    app.monitor_from_point(x, y)
        .map_err(err)?
        .or(app.primary_monitor().map_err(err)?)
        .ok_or_else(|| "no monitor".to_string())
}

/// One editor per file: refocus it if it's open, otherwise open one at 80 % of the monitor under
/// the pointer. Windows are matched on the canonical path, so the thumbnail and the tray find the
/// same one; the editor keeps `path` as given for display and the clipboard.
pub fn open_editor(app: &AppHandle, path: &std::path::Path) -> Result<(), String> {
    let canon = path.canonicalize().map_err(err)?;
    let state = app.state::<crate::AppState>();
    let existing = state
        .editors
        .lock()
        .unwrap()
        .iter()
        .find(|(_, (_, c))| *c == canon)
        .map(|(label, _)| label.clone());
    let win = match existing.and_then(|label| app.get_webview_window(&label)) {
        Some(w) => w,
        None => new_editor(app, path.to_path_buf(), canon)?,
    };
    win.show().map_err(err)?;
    win.set_focus().map_err(err)?;
    // Mutter ignores set_focus for windows opened from another app's click (Plan 1 Task 6/13).
    #[cfg(target_os = "linux")]
    force_focus(&win);
    Ok(())
}

fn new_editor(
    app: &AppHandle,
    path: std::path::PathBuf,
    canon: std::path::PathBuf,
) -> Result<WebviewWindow, String> {
    let cursor = app.cursor_position().map_err(err)?;
    let m = monitor_at(app, cursor.x, cursor.y)?;
    let s = m.scale_factor();
    let (mw, mh) = (m.size().width as f64 / s, m.size().height as f64 / s);
    let (w, h) = ((mw * 0.8).max(800.0), (mh * 0.8).max(560.0));
    let label = format!(
        "editor-{}",
        POPUP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    app.state::<crate::AppState>()
        .editors
        .lock()
        .unwrap()
        .insert(label.clone(), (path, canon));
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("editor/index.html".into()))
        .title(format!("{name} — rshot"))
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .min_inner_size(800.0, 560.0)
        .inner_size(w, h)
        .visible(false)
        .build()
        .inspect_err(|_| forget_editor(app, &label))
        .map_err(err)?;
    // The path stays readable (thumbnail::guard) exactly as long as its editor is open.
    let app2 = app.clone();
    win.on_window_event(move |e| {
        if let tauri::WindowEvent::Destroyed = e {
            forget_editor(&app2, &label);
        }
    });
    let x = m.position().x + ((m.size().width as f64 - w * s) / 2.0) as i32;
    let y = m.position().y + ((m.size().height as f64 - h * s) / 2.0) as i32;
    win.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    Ok(win)
}

fn forget_editor(app: &AppHandle, label: &str) {
    app.state::<crate::AppState>()
        .editors
        .lock()
        .unwrap()
        .remove(label);
}

/// Countdown ring centred on a desktop point (physical pixels).
pub fn show_countdown(app: &AppHandle, (cx, cy): (i32, i32)) -> Result<(), String> {
    let s = monitor_at(app, cx.into(), cy.into())?.scale_factor();
    let (w, h) = (160.0, 180.0);
    let win = popup(app, "countdown", "countdown/index.html", w, h, true)?;
    win.set_position(PhysicalPosition::new(
        cx - (w * s / 2.0) as i32,
        cy - (h * s / 2.0) as i32,
    ))
    .map_err(err)?;
    win.show().map_err(err)?;
    win.set_focus().map_err(err)
}

/// Bottom-right of the monitor under the pointer.
pub fn show_thumbnail(app: &AppHandle) -> Result<(), String> {
    let p = app.cursor_position().map_err(err)?;
    let m = monitor_at(app, p.x, p.y)?;
    let s = m.scale_factor();
    let (w, h) = (270.0, 240.0);
    let win = popup(app, "thumbnail", "thumbnail/index.html", w, h, false)?;
    let x = m.position().x + m.size().width as i32 - ((w + 6.0) * s) as i32;
    let y = m.position().y + m.size().height as i32 - ((h + 6.0) * s) as i32;
    win.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    win.show().map_err(err)
}
