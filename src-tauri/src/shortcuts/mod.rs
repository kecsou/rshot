//! Taking over / giving back the OS screenshot shortcuts.

#[cfg(target_os = "linux")]
mod gnome;
#[cfg(target_os = "linux")]
pub mod gvariant;
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(any(target_os = "windows", test))]
mod windows;

use crate::store::{self, Config};

/// The binary the bindings should run (the AppImage when running from one).
fn exe_command() -> String {
    std::env::var("APPIMAGE")
        .ok()
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .map(|p| live_path(&p.display().to_string()).into())
        })
        .unwrap_or_else(|| "rshot".into())
}

#[cfg(target_os = "linux")]
pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    gnome::take_over(cfg, &exe_command())
}

#[cfg(target_os = "linux")]
pub fn restore(cfg: &mut Config) -> Result<(), String> {
    gnome::restore(cfg)
}

#[cfg(target_os = "linux")]
pub fn outdated(cfg: &Config) -> bool {
    gnome::outdated(cfg)
}

/// Windows and macOS keep their bindings in this process: every launch takes a saved takeover
/// again.
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn outdated(cfg: &Config) -> bool {
    cfg.takeover
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn outdated(_cfg: &Config) -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    windows::take_over(cfg)
}

#[cfg(target_os = "windows")]
pub fn restore(cfg: &mut Config) -> Result<(), String> {
    windows::restore(cfg)
}

#[cfg(target_os = "macos")]
pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    macos::take_over(cfg)
}

#[cfg(target_os = "macos")]
pub fn restore(cfg: &mut Config) -> Result<(), String> {
    macos::restore(cfg)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn take_over(_cfg: &mut Config) -> Result<(), String> {
    Err("Taking over the system shortcuts on this OS arrives in a later version.".into())
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn restore(_cfg: &mut Config) -> Result<(), String> {
    Ok(())
}

/// Gives the OS its screenshot keys back but keeps the consent and the backup, so the next launch
/// takes them again: Quit, installer upgrades. Linux has nothing to give back meanwhile: GNOME's
/// bindings start rshot.
#[cfg(target_os = "windows")]
pub fn release(cfg: &Config) -> Result<(), String> {
    windows::release(cfg)
}

#[cfg(target_os = "macos")]
pub fn release(cfg: &Config) -> Result<(), String> {
    macos::release(cfg)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn release(_cfg: &Config) -> Result<(), String> {
    Ok(())
}

/// Settings is recording a new shortcut: rshot's own keys must reach it (Windows' hook lets them
/// through meanwhile, macOS unregisters its global shortcuts).
pub fn pause(on: bool) {
    #[cfg(target_os = "windows")]
    windows::pause(on);
    #[cfg(target_os = "macos")]
    macos::pause(on);
    let _ = on;
}

/// Called once at startup, before `settings::catch_up_takeover` (which takes a saved takeover
/// again where `outdated` says so): Windows installs its keyboard hook, macOS keeps the handle its
/// global shortcuts need. Linux: nothing to do.
pub fn start(app: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    windows::install_hook(app.clone());
    #[cfg(target_os = "macos")]
    macos::init(app.clone());
    let _ = app;
}

/// Gives the shortcuts back and records that in config.toml.
pub fn restore_and_save(c: &mut Config) -> Result<(), String> {
    restore(c)?;
    c.takeover = false;
    store::save_config(c).map_err(|e| e.to_string())
}

/// `rshot restore-shortcuts [--keep-consent]` when no daemon answered (a running daemon does it
/// itself, so its in-memory config doesn't write the old takeover back later). Used by
/// uninstallers; `--keep-consent` (`release`) by installer upgrades.
pub fn restore_from_cli(keep_consent: bool) -> Result<(), String> {
    // No config = rshot never ran for this user (prerm runs this for everyone logged in,
    // gdm included): nothing to restore, and no config.toml to create. On macOS it may also be a
    // lost config.toml, whose backup lives on in rshot's own defaults (macos.rs): that goes back
    // and is removed, still without saving a config.
    if !store::config_path().exists() {
        #[cfg(target_os = "macos")]
        return restore(&mut Config::default());
        #[cfg(not(target_os = "macos"))]
        return Ok(());
    }
    let mut c = store::load_config();
    if keep_consent {
        release(&c)
    } else {
        restore_and_save(&mut c)
    }
}

/// Once a package upgrade has replaced the running binary, Linux reports it as
/// `/usr/bin/rshot (deleted)`; the new one is at the same path.
fn live_path(p: &str) -> &str {
    p.strip_suffix(" (deleted)").unwrap_or(p)
}

/// `"exe" sub`, quoted by the Desktop Entry Exec rules GNOME applies to custom shortcuts
/// (`g_app_info_create_from_commandline`): `%` is a field code, and `"`, `` ` ``, `$`, `\`
/// need a backslash inside double quotes.
pub fn command_for(exe: &str, sub: &str) -> String {
    let mut q = String::new();
    for c in exe.chars() {
        match c {
            '"' | '`' | '$' | '\\' => q.extend(['\\', c]),
            '%' => q.push_str("%%"),
            _ => q.push(c),
        }
    }
    format!("\"{q}\" {sub}")
}

/// (shortcut, command) pairs to bind by hand on desktops rshot can't configure (unbound actions
/// left out).
pub fn manual_commands(c: &Config) -> Vec<(String, String)> {
    let exe = exe_command();
    let all = [
        (c.shortcuts.area.clone(), command_for(&exe, "capture area")),
        (
            c.shortcuts.screen.clone(),
            command_for(&exe, "capture screen"),
        ),
        (
            c.shortcuts.window.clone(),
            command_for(&exe, "capture window"),
        ),
        (c.shortcuts.record.clone(), command_for(&exe, "record")),
    ];
    all.into_iter().filter(|(k, _)| !k.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_replaced_binary_keeps_its_path() {
        assert_eq!(
            super::live_path("/usr/bin/rshot (deleted)"),
            "/usr/bin/rshot"
        );
        assert_eq!(super::live_path("/usr/bin/rshot"), "/usr/bin/rshot");
    }

    #[test]
    fn commands_survive_exec_parsing() {
        assert_eq!(
            super::command_for("/usr/bin/rshot", "capture area"),
            r#""/usr/bin/rshot" capture area"#
        );
        assert_eq!(
            super::command_for(r#"/my apps/$x"`\100%/rshot"#, "capture screen"),
            r#""/my apps/\$x\"\`\\100%%/rshot" capture screen"#
        );
    }
}
