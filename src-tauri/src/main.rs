#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod cli;
mod clipboard;
mod overlay;
mod pipeline;
mod store;
mod ui;

use tauri::{AppHandle, RunEvent};

/// Error adapter for IPC: every command error is a `String`.
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub struct AppState {
    pub config: std::sync::Mutex<store::Config>,
    pub clipboard: clipboard::Clipboard,
    pub last_capture: std::sync::Mutex<Option<std::path::PathBuf>>,
    pub session: std::sync::Mutex<Option<overlay::Session>>,
    pub next_token: std::sync::atomic::AtomicU64,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: std::sync::Mutex::new(store::load_config()),
            clipboard: clipboard::Clipboard::spawn(),
            last_capture: std::sync::Mutex::new(None),
            session: std::sync::Mutex::new(None),
            next_token: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = cli::parse(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });

    let app = tauri::Builder::default()
        // Must stay the first plugin: a second `rshot …` forwards its argv here and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            match cli::parse(argv.get(1..).unwrap_or_default()) {
                Ok(cmd) => dispatch(app, cmd),
                Err(e) => eprintln!("rshot: {e}"),
            }
        }))
        .manage(AppState::new())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            overlay::overlay_info,
            overlay::overlay_frame,
            overlay::overlay_ready,
            overlay::overlay_activate,
            overlay::overlay_cancel,
            overlay::overlay_capture,
        ])
        .setup(move |app| {
            ui::create_tray(app.handle())?;
            ui::ensure_overlays(app.handle())?;
            dispatch(app.handle(), cmd);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build rshot");

    app.run(|_app, event| {
        // Live in the tray: closing the last window must not quit; only app.exit(code) does.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}

pub fn dispatch(app: &AppHandle, cmd: cli::Cmd) {
    eprintln!("rshot: dispatch {cmd:?}");
    use cli::Cmd::*;
    let result = match cmd {
        Daemon | RestoreShortcuts => Ok(()),
        CaptureArea => overlay::start(app, "area"),
        CaptureScreen => pipeline::capture_screen_now(app),
        CaptureWindow => pipeline::capture_window_now(app),
    };
    if let Err(e) = result {
        pipeline::notify(app, &e);
    }
}
