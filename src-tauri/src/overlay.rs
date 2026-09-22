//! Overlay session: frozen frames per monitor, the frame IPC, and capture targets.

use crate::{
    capture::{self, Frame, WinRect},
    err, pipeline, store, ui, AppState,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::Ordering,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Response, AppHandle, Emitter, Manager, State, WebviewWindow};

pub struct Session {
    pub token: u64,
    pub mode: String,
    pub frames: Vec<Frame>,
    pub active: usize,
    pub started: Instant,
}

/// Image pixels of one frame.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Area { rect: Rect },
    Window { id: u32, rect: Rect },
    Screen,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OverlayOptions {
    pub timer_secs: u8,
    pub show_thumbnail: bool,
    pub remember_selection: bool,
    pub show_pointer: bool,
    pub screenshots_dir: String,
}

impl OverlayOptions {
    pub fn from_config(c: &store::Config) -> Self {
        Self {
            timer_secs: c.timer_secs,
            show_thumbnail: c.show_thumbnail,
            remember_selection: c.remember_selection,
            show_pointer: c.show_pointer,
            screenshots_dir: store::screenshots_dir(c).display().to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct OverlayInfo {
    token: u64,
    mode: String,
    index: usize,
    active: bool,
    width: u32,
    height: u32,
    windows: Vec<WinRect>,
    selection: Option<Rect>,
    hints: bool,
    options: OverlayOptions,
}

/// The hint bar shows on the first few overlays only.
const HINT_SESSIONS: u32 = 5;

fn epoch_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Grabs every monitor and asks each overlay page to show its frame.
pub fn start(app: &AppHandle, mode: &str) -> Result<(), String> {
    let started = Instant::now();
    eprintln!("rshot: overlay requested at {}", epoch_ms());
    let state = app.state::<AppState>();
    let show_pointer = state.config.lock().unwrap().show_pointer;
    let frames = capture::grab_all(show_pointer)?;
    eprintln!("rshot: grabbed {} monitor(s) in {} ms", frames.len(), started.elapsed().as_millis());
    let pos = app.cursor_position().map_err(err)?;
    let active = capture::frame_at(&frames, pos.x as i32, pos.y as i32);
    ui::place_overlays(app, &frames)?;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    *state.session.lock().unwrap() = Some(Session { token, mode: mode.into(), frames, active, started });
    {
        let mut c = state.config.lock().unwrap();
        if c.hints_shown <= HINT_SESSIONS {
            c.hints_shown += 1;
            let _ = store::save_config(&c);
        }
    }
    app.emit("overlay:show", token).map_err(err)
}

#[tauri::command]
pub fn overlay_info(window: WebviewWindow, state: State<'_, AppState>) -> Option<OverlayInfo> {
    let index = ui::overlay_index(window.label())?;
    let session = state.session.lock().unwrap();
    let s = session.as_ref()?;
    let f = s.frames.get(index)?;
    let c = state.config.lock().unwrap();
    let selection = c
        .last_selection
        .as_ref()
        .filter(|l| c.remember_selection && l.monitor == f.name)
        .map(|l| Rect { x: l.x.into(), y: l.y.into(), w: l.w.into(), h: l.h.into() });
    Some(OverlayInfo {
        token: s.token,
        mode: s.mode.clone(),
        index,
        active: index == s.active,
        width: f.image.width(),
        height: f.image.height(),
        windows: capture::windows_on(f),
        selection,
        hints: c.hints_shown <= HINT_SESSIONS,
        options: OverlayOptions::from_config(&c),
    })
}

/// Raw RGBA of this overlay's frame (width/height come from overlay_info).
#[tauri::command]
pub async fn overlay_frame(app: AppHandle, window: WebviewWindow) -> Result<Response, String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    let state = app.state::<AppState>();
    let session = state.session.lock().unwrap();
    let f = session.as_ref().and_then(|s| s.frames.get(index)).ok_or("no frame")?;
    Ok(Response::new(f.image.as_raw().clone()))
}

/// The page painted its frame: show it (and focus the one under the pointer).
/// Mutter focuses each overlay as it maps, and tao ignores set_focus until GTK has shown the
/// window, so only the overlay under the pointer may take focus (overlay_activate moves it).
#[tauri::command]
pub fn overlay_ready(window: WebviewWindow, state: State<'_, AppState>, token: u64) -> Result<(), String> {
    let session = state.session.lock().unwrap();
    let Some(s) = session.as_ref().filter(|s| s.token == token) else {
        return Ok(());
    };
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    window.set_focusable(index == s.active).map_err(err)?;
    window.show().map_err(err)?;
    if index == s.active {
        window.set_focus().map_err(err)?;
        #[cfg(target_os = "linux")]
        ui::force_focus(&window);
        eprintln!("rshot: overlay visible at {} ({} ms after trigger)", epoch_ms(), s.started.elapsed().as_millis());
        // The other overlays wait for this before fetching their frames, so they don't slow it down.
        window.emit("overlay:primary-ready", token).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_activate(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>, token: u64) -> Result<(), String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    if let Some(s) = state.session.lock().unwrap().as_mut().filter(|s| s.token == token) {
        s.active = index;
    }
    let _ = window.set_focusable(true);
    let _ = window.set_focus();
    app.emit("overlay:active", index).map_err(err)
}

/// Async on purpose: hiding the overlays synchronously on the main thread, inside the Esc
/// key's IPC call, made the next overlay swallow its first key press (seen on GNOME/X11).
#[tauri::command]
pub async fn overlay_cancel(app: AppHandle) {
    let state = app.state::<AppState>();
    state.session.lock().unwrap().take();
    ui::hide_overlays(&app);
}

#[tauri::command]
pub async fn overlay_capture(app: AppHandle, window: WebviewWindow, token: u64, target: Target) -> Result<(), String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    let session = {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock().unwrap();
        match guard.take() {
            Some(s) if s.token == token => s,
            other => {
                *guard = other;
                return Ok(());
            }
        }
    };
    ui::hide_overlays(&app);
    let result = capture_from(&app, session, index, target);
    if let Err(e) = &result {
        pipeline::notify(&app, e);
    }
    result
}

fn capture_from(app: &AppHandle, session: Session, index: usize, target: Target) -> Result<(), String> {
    let frame = session.frames.into_iter().nth(index).ok_or("that monitor is gone")?;
    if let Target::Area { rect } = target {
        remember(app, &frame.name, rect);
    }
    let (w, h) = frame.image.dimensions();
    let clamp = |r: Rect| capture::clamp_rect(r.x, r.y, r.w, r.h, w, h).ok_or_else(|| "empty selection".to_string());
    let img = match target {
        Target::Screen => frame.image,
        Target::Area { rect } => capture::crop(&frame.image, clamp(rect)?),
        // Window pixels come from the window itself (even if covered); fall back to the frozen frame.
        Target::Window { id, rect } => match capture::window_image(id) {
            Ok(img) => img,
            Err(_) => capture::crop(&frame.image, clamp(rect)?),
        },
    };
    pipeline::finish_capture(app, img).map(|_| ())
}

fn remember(app: &AppHandle, monitor: &str, r: Rect) {
    let state = app.state::<AppState>();
    let mut c = state.config.lock().unwrap();
    c.last_selection = Some(store::Selection {
        monitor: monitor.into(),
        x: r.x.max(0.0) as u32,
        y: r.y.max(0.0) as u32,
        w: r.w.max(0.0) as u32,
        h: r.h.max(0.0) as u32,
    });
    let _ = store::save_config(&c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_parse_from_the_ts_shapes() {
        let t = |s: &str| serde_json::from_str::<Target>(s).unwrap();
        let r = Rect { x: 1.5, y: 2.0, w: 3.0, h: 4.0 };
        let rect = r#""rect":{"x":1.5,"y":2,"w":3,"h":4}"#;
        assert!(matches!(t(&format!(r#"{{"kind":"area",{rect}}}"#)), Target::Area { rect } if rect == r));
        assert!(matches!(t(&format!(r#"{{"kind":"window","id":7,{rect}}}"#)), Target::Window { id: 7, rect } if rect == r));
        assert!(matches!(t(r#"{"kind":"screen"}"#), Target::Screen));
        assert_eq!(ui::overlay_index("overlay-2"), Some(2));
        assert_eq!(ui::overlay_index("settings"), None);
    }
}
