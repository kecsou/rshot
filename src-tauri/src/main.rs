#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod cli;
mod clipboard;
mod overlay;
mod pipeline;
mod settings;
mod shortcuts;
mod store;
mod thumbnail;
mod ui;

use tauri::{AppHandle, Manager, RunEvent};

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
    pub pending: std::sync::Mutex<Option<overlay::Pending>>,
    pub thumb: std::sync::Mutex<Option<thumbnail::Thumb>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: std::sync::Mutex::new(store::load_config()),
            clipboard: clipboard::Clipboard::spawn(),
            last_capture: std::sync::Mutex::new(None),
            session: std::sync::Mutex::new(None),
            next_token: std::sync::atomic::AtomicU64::new(1),
            pending: std::sync::Mutex::new(None),
            thumb: std::sync::Mutex::new(None),
        }
    }
}

fn main() {
    // WebKitGTK's DMA-BUF renderer crashes or renders blank on some NVIDIA/EGL setups; the
    // shared-memory path is plenty for rshot's small windows. Respect an explicit user setting.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = cli::parse(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    // A forwarded restore-shortcuts runs in the daemon, the one config writer. The call returning
    // means it was handled; a failure there shows as a notification, not as this exit code.
    #[cfg(target_os = "linux")]
    if cmd != cli::Cmd::Daemon && forward_to_daemon() {
        return;
    }
    if cmd == cli::Cmd::RestoreShortcuts {
        std::process::exit(match shortcuts::restore_from_cli() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("rshot: {e}");
                1
            }
        });
    }
    // Frame-sized buffers (8-16 MB) must go back to the OS when freed. glibc's dynamic mmap
    // threshold otherwise rises to the first freed frame's size and parks later frames in
    // per-thread arenas, whose free tops malloc_trim can't release (~16 MB per arena).
    #[cfg(target_os = "linux")]
    // SAFETY: plain allocator tuning, before any other thread exists.
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 1 << 20);
    }

    let app = tauri::Builder::default()
        // Must stay the first plugin: a second `rshot …` forwards its argv here and exits.
        .plugin(tauri_plugin_single_instance::init(
            |app, argv, _cwd| match cli::parse(argv.get(1..).unwrap_or_default()) {
                Ok(cmd) => dispatch(app, cmd),
                Err(e) => eprintln!("rshot: {e}"),
            },
        ))
        .manage(AppState::new())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            overlay::overlay_info,
            overlay::overlay_frame,
            overlay::overlay_ready,
            overlay::overlay_activate,
            overlay::overlay_cancel,
            overlay::overlay_capture,
            overlay::set_overlay_options,
            overlay::pick_folder,
            overlay::countdown_info,
            overlay::countdown_done,
            overlay::countdown_cancel,
            thumbnail::thumbnail_info,
            thumbnail::read_capture,
            thumbnail::reveal_capture,
            thumbnail::open_capture,
            thumbnail::delete_capture,
            thumbnail::retry_copy,
            thumbnail::dismiss_thumbnail,
            settings::get_settings,
            settings::set_settings,
            settings::onboarding_choice,
            settings::open_config,
            settings::close_window,
        ])
        .setup(move |app| {
            ui::create_tray(app.handle())?;
            ui::ensure_overlays(app.handle())?;
            if !app.state::<AppState>().config.lock().unwrap().onboarded {
                ui::open_onboarding(app.handle())?;
            }
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

/// The single-instance plugin's D-Bus name: tauri.conf.json's identifier + ".SingleInstance".
#[cfg(target_os = "linux")]
const SINGLE_INSTANCE_NAME: &str = "io.github.kecsou.rshot.SingleInstance";

/// Hands argv to a running daemon through the single-instance plugin's own D-Bus interface,
/// before Tauri/GTK start. The plugin would forward the same call, but only after the whole app
/// has initialised: doing it here saves ~45 ms per key press (measured).
/// `false` = no daemon answered; the caller then starts as the daemon.
#[cfg(target_os = "linux")]
fn forward_to_daemon() -> bool {
    let path = format!("/{}", SINGLE_INSTANCE_NAME.replace('.', "/"));
    let argv: Vec<String> = std::env::args().collect();
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .display()
        .to_string();
    zbus::blocking::Connection::session()
        .and_then(|c| {
            c.call_method(
                Some(SINGLE_INSTANCE_NAME),
                path.as_str(),
                Some("org.SingleInstance.DBus"),
                "ExecuteCallback",
                &(argv, cwd),
            )
        })
        .is_ok()
}

pub fn dispatch(app: &AppHandle, cmd: cli::Cmd) {
    eprintln!("rshot: dispatch {cmd:?}");
    use cli::Cmd::*;
    let result = match cmd {
        Daemon => Ok(()),
        RestoreShortcuts => {
            shortcuts::restore_and_save(&mut app.state::<AppState>().config.lock().unwrap())
        }
        CaptureArea => overlay::start(app, "area"),
        CaptureScreen => pipeline::capture_screen_now(app),
        CaptureWindow => pipeline::capture_window_now(app),
    };
    if let Err(e) = result {
        pipeline::notify(app, &e);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn single_instance_name_follows_the_bundle_identifier() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            format!("{}.SingleInstance", conf["identifier"].as_str().unwrap()),
            super::SINGLE_INSTANCE_NAME
        );
    }
}
