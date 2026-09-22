//! Windows and tray.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit = MenuItem::with_id(app, "quit", "Quit rshot", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundle icon is configured"))
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            if e.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;
    Ok(())
}
