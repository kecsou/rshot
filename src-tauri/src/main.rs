#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod store;
mod ui;

use tauri::{AppHandle, RunEvent};

/// Error adapter for IPC: every command error is a `String`.
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub struct AppState {
    pub config: std::sync::Mutex<store::Config>,
}

impl AppState {
    fn new() -> Self {
        Self { config: std::sync::Mutex::new(store::load_config()) }
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
        .setup(move |app| {
            ui::create_tray(app.handle())?;
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

pub fn dispatch(_app: &AppHandle, cmd: cli::Cmd) {
    eprintln!("rshot: dispatch {cmd:?}");
}
