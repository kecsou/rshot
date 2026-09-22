//! Windows and tray.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

use crate::cli::Cmd;
use std::time::Duration;

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
