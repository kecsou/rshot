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

/// The tray's Record item, relabelled "Stop Recording" while recording (appindicator trays
/// can't be clicked on Linux, so the menu is the tray's stop button).
pub struct TrayRecord(pub MenuItem<tauri::Wry>);

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let sep = || tauri::menu::PredefinedMenuItem::separator(app);
    let record = MenuItem::with_id(
        app,
        "record",
        RECORD_LABEL,
        crate::recorder::unavailable(app).is_none(),
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &item("area", "Capture Area")?,
            &item("screen", "Capture Screen")?,
            &item("window", "Capture Window")?,
            &record,
            &sep()?,
            &item("last", "Open Last Capture")?,
            &item("folder", "Open Screenshots Folder")?,
            &sep()?,
            &item("settings", "Settings…")?,
            &item("quit", "Quit rshot")?,
        ],
    )?;
    TrayIconBuilder::with_id(TRAY)
        .icon(
            app.default_window_icon()
                .cloned()
                .expect("bundle icon is configured"),
        )
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            let result = match e.id.as_ref() {
                "quit" => return quit(app),
                "settings" => open_settings(app),
                "last" => open_last(app),
                "folder" => open_folder(app),
                id => {
                    let cmd = match id {
                        "area" => Cmd::CaptureArea,
                        "screen" => Cmd::CaptureScreen,
                        "window" => Cmd::CaptureWindow,
                        "record" => Cmd::Record,
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
    app.manage(TrayRecord(record));
    Ok(())
}

const TRAY: &str = "main";
const RECORD_LABEL: &str = "Record Screen…";

/// `■ m:ss` next to the tray icon; `None` removes it.
pub fn set_tray_timer(app: &AppHandle, secs: Option<u64>) {
    if let Some(tray) = app.tray_by_id(TRAY) {
        let _ = tray.set_title(secs.map(|s| format!("■ {}:{:02}", s / 60, s % 60)));
    }
}

/// While recording: red tray icon, "Stop Recording", timer; for an area, also a dashed frame just
/// outside it and the control pill. A full-screen recording would film both, so it gets neither.
pub fn recording_started(
    app: &AppHandle,
    r: crate::recorder::Region,
    full: bool,
) -> Result<(), String> {
    if let Some(item) = app.try_state::<TrayRecord>() {
        let _ = item.0.set_text("Stop Recording");
    }
    if let Some(tray) = app.tray_by_id(TRAY) {
        let _ = tray.set_icon(Some(recording_icon()));
    }
    set_tray_timer(app, Some(0));
    if full {
        return Ok(());
    }
    let m = monitor_at(app, r.x.into(), r.y.into())?;
    let s = m.scale_factor();
    // A frame that failed halfway (e.g. not click-through) goes; the pill comes up regardless.
    let frame = show_frame(app, r, s);
    if frame.is_err() {
        close_prefix(app, "recframe");
    }
    let pill = show_pill(app, r, &m);
    if pill.is_err() {
        close_prefix(app, "pill");
    }
    recording_ui_up(app);
    pill.and(frame)
}

/// The control pill, above the area's frame or below it, within the monitor's work area.
fn show_pill(
    app: &AppHandle,
    r: crate::recorder::Region,
    m: &tauri::Monitor,
) -> Result<(), String> {
    let s = m.scale_factor();
    let (pw, ph) = (300.0, 52.0);
    let pill = popup(app, "pill", "pill/index.html", pw, ph, false)?;
    // Within the work area: clear of the top bar and the dock.
    let wa = m.work_area();
    let (x, y) = pill_position(
        r,
        frame_margin(s),
        ((pw * s) as i32, (ph * s) as i32, (4.0 * s) as i32),
        (
            wa.position.x,
            wa.position.y,
            wa.size.width as i32,
            wa.size.height as i32,
        ),
    );
    pill.set_position(PhysicalPosition::new(x, y))
        .map_err(err)?;
    #[cfg(target_os = "linux")]
    unmanaged(&pill)?;
    pill.show().map_err(err)
}

/// The dashed frame around area `r` on a monitor of scale `s`: transparent and click-through.
fn show_frame(app: &AppHandle, r: crate::recorder::Region, s: f64) -> Result<(), String> {
    let (x, y, w, h) = frame_rect(r, s);
    let frame = popup(app, "recframe", "recframe/index.html", 100.0, 100.0, false)?;
    frame
        .set_position(PhysicalPosition::new(x, y))
        .map_err(err)?;
    frame.set_size(PhysicalSize::new(w, h)).map_err(err)?;
    #[cfg(target_os = "linux")]
    unmanaged(&frame)?;
    // After show: tao sets the empty input shape on the GdkWindow, which a never-shown window
    // doesn't have yet (it unwraps it, aborting rshot).
    frame.show().map_err(err)?;
    frame.set_ignore_cursor_events(true).map_err(err)
}

/// How far the frame window reaches out of the area, in physical px: recframe.css draws within
/// 8 CSS px of the window's edge.
fn frame_margin(s: f64) -> i32 {
    (8.0 * s).round() as i32
}

/// The frame window's `(x, y, w, h)` in physical px for area `r` on a monitor of scale `s`.
fn frame_rect(r: crate::recorder::Region, s: f64) -> (i32, i32, u32, u32) {
    let m = frame_margin(s);
    (r.x - m, r.y - m, r.w + 2 * m as u32, r.h + 2 * m as u32)
}

/// Makes a not-yet-shown window override-redirect, i.e. not managed by the window manager, for
/// the recording frame and pill. GNOME animates managed windows appearing and going (hiding one
/// counts as going) by scaling and fading them: the frame's line would sweep across the recorded
/// area, and a hidden pill would still be fading out in a screenshot. Mutter could also push a
/// frame larger than the monitor back onto it, and focus the pill when it's shown again. Queued
/// before `show()`, so it runs first (Tauri handles both, in order, on the main thread, and the
/// window's position and size are applied later, before it maps).
#[cfg(target_os = "linux")]
fn unmanaged(w: &WebviewWindow) -> Result<(), String> {
    let w2 = w.clone();
    w.run_on_main_thread(move || {
        if let Ok(gw) = w2.gtk_window() {
            gw.realize();
            if let Some(gdk) = gw.window() {
                gdk.set_override_redirect(true);
            }
        }
    })
    .map_err(err)
}

/// The pill's top-left (physical px) for area `r` framed `margin` out: `gap` above the frame's
/// top-right corner, else below the frame, and always inside the monitor's work area `(x, y, w, h)`
/// (a monitor-tall area gets it over its bottom edge, in the video, rather than nowhere).
fn pill_position(
    r: crate::recorder::Region,
    margin: i32,
    (pw, ph, gap): (i32, i32, i32),
    (mx, my, mw, mh): (i32, i32, i32, i32),
) -> (i32, i32) {
    let above = r.y - margin - gap - ph;
    let y = if above >= my {
        above
    } else {
        r.y + r.h as i32 + margin + gap
    };
    let x = r.x + r.w as i32 - pw;
    // min then max, not clamp: no panic on a monitor smaller than the pill.
    (x.min(mx + mw - pw).max(mx), y.min(my + mh - ph).max(my))
}

pub fn recording_stopped(app: &AppHandle) {
    kept_hidden().1.clear();
    if let Some(item) = app.try_state::<TrayRecord>() {
        let _ = item.0.set_text(RECORD_LABEL);
    }
    if let Some(tray) = app.tray_by_id(TRAY) {
        let _ = tray.set_icon(app.default_window_icon().cloned());
    }
    set_tray_timer(app, None);
    close_prefix(app, "recframe");
    close_prefix(app, "pill");
}

/// A red (#ff453a) dot, 32×32 RGBA.
fn recording_icon() -> tauri::image::Image<'static> {
    const N: u32 = 32;
    let rgba = (0..N * N)
        .flat_map(|i| {
            let (x, y) = ((i % N) as f32 - 15.5, (i / N) as f32 - 15.5);
            if x * x + y * y <= 13.0 * 13.0 {
                [0xff, 0x45, 0x3a, 0xff]
            } else {
                [0; 4]
            }
        })
        .collect();
    tauri::image::Image::new_owned(rgba, N, N)
}

/// Set while a Quit is saving the recording; that Quit carries on by itself afterwards.
static QUITTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Open editors are asked to close first (each prompts for unsaved changes); Quit exits once
/// none is left, so after that a second Quit does. A recording is stopped and saved first, off the
/// main thread (ffmpeg would otherwise outlive rshot and record until the disk is full), and a
/// stop or discard already under way (pill, tray, shortcut) is waited for; a Quit repeated
/// meanwhile is ignored, as exiting would cut the save short.
fn quit(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    // A countdown ending now must not start a recording.
    app.state::<crate::AppState>()
        .pending_rec
        .lock()
        .unwrap()
        .take();
    if QUITTING.load(Ordering::Acquire) {
        return;
    }
    if crate::recorder::is_recording(app) || crate::recorder::is_ending() {
        QUITTING.store(true, Ordering::Release);
        let app = app.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::recorder::stop(&app) {
                crate::pipeline::notify(&app, &e);
            }
            QUITTING.store(false, Ordering::Release);
            quit(&app);
        });
        return;
    }
    let labels: Vec<String> = app
        .state::<crate::AppState>()
        .editors
        .lock()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let editors: Vec<WebviewWindow> = labels
        .iter()
        .filter_map(|l| app.get_webview_window(l))
        .collect();
    if editors.is_empty() {
        return app.exit(0);
    }
    for w in editors {
        // Raised first, so an unsaved-changes prompt can't sit out of sight.
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        #[cfg(target_os = "linux")]
        force_focus(&w);
        let _ = w.close(); // a close request, like the window's own close: the editor decides
    }
}

fn open_last(app: &AppHandle) -> Result<(), String> {
    let last = app
        .state::<crate::AppState>()
        .last_capture
        .lock()
        .unwrap()
        .clone();
    let path = last.ok_or("No capture yet")?;
    let canon = path
        .canonicalize()
        .map_err(|_| "The last capture no longer exists")?;
    open_editor(app, &path, canon)
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
    // After the overlays: a recording's frame and pill come back, but never over a frozen frame.
    let hold = OVERLAY_HOLD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    drop(hold);
}

/// The grab that opened the overlay keeps the recording's frame and pill hidden while it's up (they
/// are override-redirect: they'd sit above it, the pill clickable). `hide_overlays` lets go.
pub fn hide_recording_ui_while_overlay(hidden: Hidden) {
    let old = OVERLAY_HOLD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .replace(hidden);
    drop(old);
}

static OVERLAY_HOLD: std::sync::Mutex<Option<Hidden>> = std::sync::Mutex::new(None);

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
/// open overlay session, a countdown (dropping its pending capture or recording), the thumbnail
/// card, and a recording's frame and pill, all always-on-top. Those two are only hidden: they come
/// back once the returned guard and every other one drop, so drop it right after the grab. Waits
/// for the compositor only when something was up. Not on the main thread.
pub fn clear_own_ui(app: &AppHandle) -> Hidden {
    // First: taking the overlay session below lets go of its own hold, which mustn't show them.
    let recording = {
        let mut kept = kept_hidden();
        kept.0 += 1;
        for w in &kept.1 {
            let _ = w.hide();
        }
        !kept.1.is_empty()
    };
    let state = app.state::<crate::AppState>();
    let overlay = state.session.lock().unwrap().take().is_some();
    if overlay {
        hide_overlays(app);
    }
    state.pending.lock().unwrap().take();
    state.pending_rec.lock().unwrap().take();
    let countdown = close_prefix(app, "countdown");
    let thumbnail = close_prefix(app, "thumbnail");
    if overlay || countdown || thumbnail || recording {
        std::thread::sleep(Duration::from_millis(150));
    }
    Hidden(())
}

/// The recording's frame and pill once they're up, and how many grabs and open overlays keep them
/// hidden: the last of those to end shows them again. Held across the hide and show calls, which
/// only queue work for the main thread, so an earlier grab's show can't land in a later one.
static KEPT_HIDDEN: std::sync::Mutex<(usize, Vec<WebviewWindow>)> =
    std::sync::Mutex::new((0, Vec::new()));

fn kept_hidden() -> std::sync::MutexGuard<'static, (usize, Vec<WebviewWindow>)> {
    KEPT_HIDDEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The frame and pill have just been shown: under a grab or an overlay they hide again at once,
/// and come back with it. (Not before: a guard mustn't show them while they're being set up.)
fn recording_ui_up(app: &AppHandle) {
    let mut kept = kept_hidden();
    kept.1 = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("recframe-") || label.starts_with("pill-"))
        .map(|(_, w)| w)
        .collect();
    if kept.0 > 0 {
        for w in &kept.1 {
            let _ = w.hide();
        }
    }
}

/// One hold on a recording's frame and pill (`clear_own_ui`): the last to drop shows them again. (A
/// recording that ended meanwhile has destroyed them; showing those fails, harmlessly.)
#[must_use = "dropping it shows the recording's frame and pill again: keep it until the grab is done"]
pub struct Hidden(());

impl Drop for Hidden {
    fn drop(&mut self) {
        let mut kept = kept_hidden();
        kept.0 -= 1;
        if kept.0 == 0 {
            for w in &kept.1 {
                let _ = w.show();
            }
        }
    }
}

/// A small undecorated, transparent, always-on-top window (thumbnail, countdown, recording frame and pill).
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

/// The monitor containing a desktop point in physical px (the cursor, a region), else the primary.
/// Not `monitor_from_point`: on Linux that takes GDK logical coordinates, so at scale 2 it finds
/// the wrong monitor, or none.
fn monitor_at(app: &AppHandle, x: f64, y: f64) -> Result<tauri::Monitor, String> {
    let mut all = app.available_monitors().map_err(err)?;
    let i = crate::capture::rect_at(
        all.iter().map(|m| {
            let (p, s) = (m.position(), m.size());
            (p.x, p.y, s.width, s.height)
        }),
        x as i32,
        y as i32,
    );
    match i {
        Some(i) => Ok(all.swap_remove(i)),
        None => app
            .primary_monitor()
            .map_err(err)?
            .or_else(|| all.into_iter().next())
            .ok_or_else(|| "no monitor".to_string()),
    }
}

/// One editor per file: refocus it if it's open, otherwise open one at 80 % of the monitor under
/// the pointer. Windows are matched on `canon` (`path` canonicalized by the caller), so the
/// thumbnail and the tray find the same one; the editor keeps `path` as given for display and the
/// clipboard. The thumbnail card goes once the editor is up (on failure it stays, to retry from).
pub fn open_editor(
    app: &AppHandle,
    path: &std::path::Path,
    canon: std::path::PathBuf,
) -> Result<(), String> {
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
    let _ = win.unminimize();
    win.show().map_err(err)?;
    win.set_focus().map_err(err)?;
    // Mutter ignores set_focus for windows opened from another app's click (Plan 1 Task 6/13).
    #[cfg(target_os = "linux")]
    force_focus(&win);
    close_prefix(app, "thumbnail");
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
    let page = if crate::thumbnail::is_mp4(&canon) {
        // Its page plays it from the loopback server, for as long as this window is open. Without
        // a stream the page still opens, and says it can't play the video (Copy, Delete work).
        if let Err(e) = crate::stream::share(&label, canon.clone()) {
            eprintln!("rshot: can't stream {}: {e}", canon.display());
        }
        "video/index.html"
    } else {
        "editor/index.html"
    };
    app.state::<crate::AppState>()
        .editors
        .lock()
        .unwrap()
        .insert(label.clone(), (path, canon));
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(page.into()))
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
    crate::stream::revoke(label);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::Region;

    #[test]
    fn the_dashed_line_stays_outside_the_area() {
        // recframe.css: the line is `inset: 3px` + `border: 2px`, i.e. 3..5 CSS px in from the
        // window's edge, and a CSS px is `s` physical px.
        const LINE_END: f64 = 5.0;
        let a = Region {
            x: 1000,
            y: 300,
            w: 641,
            h: 377,
        };
        for s in [1.0, 1.25, 2.0] {
            let (x, y, w, h) = frame_rect(a, s);
            let (x, y, w, h) = (x as f64, y as f64, w as f64, h as f64);
            let inner = LINE_END * s;
            // At least 2 physical px clear of the area, which absorbs tao's logical rounding.
            assert!(x + inner <= a.x as f64 - 2.0, "left at {s}");
            assert!(y + inner <= a.y as f64 - 2.0, "top at {s}");
            assert!(
                x + w - inner >= (a.x + a.w as i32) as f64 + 2.0,
                "right at {s}"
            );
            assert!(
                y + h - inner >= (a.y + a.h as i32) as f64 + 2.0,
                "bottom at {s}"
            );
        }
        assert_eq!(frame_rect(a, 1.0), (992, 292, 657, 393));
        assert_eq!(frame_rect(a, 2.0), (984, 284, 673, 409));
    }

    #[test]
    fn pill_sits_outside_the_area_and_in_the_work_area() {
        let area = |x, y, w, h| Region { x, y, w, h };
        let pill = (300, 52, 4);
        let mon = (1920, 0, 2560, 1600);
        // Room above: right-aligned, clear of the 8 px frame.
        assert_eq!(
            pill_position(area(2500, 400, 600, 300), 8, pill, mon),
            (2800, 336)
        );
        // None above: below the frame.
        assert_eq!(
            pill_position(area(2500, 20, 600, 300), 8, pill, mon),
            (2800, 332)
        );
        // Monitor-tall: pulled up onto the monitor.
        assert_eq!(
            pill_position(area(2000, 0, 800, 1600), 8, pill, mon),
            (2500, 1548)
        );
        // Narrower than the pill at the monitor's left edge: pulled right onto it.
        assert_eq!(
            pill_position(area(1920, 500, 100, 100), 8, pill, mon),
            (1920, 436)
        );
        // A monitor smaller than the pill: its top-left corner.
        assert_eq!(
            pill_position(area(10, 10, 50, 50), 8, pill, (0, 0, 200, 40)),
            (0, 0)
        );
        // Under a 32 px top bar (the work area starts at y 32): below instead of under the bar.
        assert_eq!(
            pill_position(area(2500, 60, 600, 300), 8, pill, (1970, 32, 2510, 1568)),
            (2800, 372)
        );
        // Scale 2: everything doubles.
        assert_eq!(
            pill_position(area(2500, 400, 600, 300), 16, (600, 104, 8), mon),
            (2500, 272)
        );
    }
}
