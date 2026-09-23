//! Taking over / giving back the OS screenshot shortcuts.

#[cfg(target_os = "linux")]
mod gnome;
#[cfg(target_os = "linux")]
pub mod gvariant;

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

#[cfg(not(target_os = "linux"))]
pub fn take_over(_cfg: &mut Config) -> Result<(), String> {
    Err("Taking over the system shortcuts on this OS arrives in a later version.".into())
}

#[cfg(not(target_os = "linux"))]
pub fn restore(_cfg: &mut Config) -> Result<(), String> {
    Ok(())
}

/// Gives the shortcuts back and records that in config.toml.
pub fn restore_and_save(c: &mut Config) -> Result<(), String> {
    restore(c)?;
    c.takeover = false;
    store::save_config(c).map_err(|e| e.to_string())
}

/// `rshot restore-shortcuts` when no daemon answered (a running daemon does it itself, so its
/// in-memory config doesn't write the old takeover back later). Used by uninstallers.
pub fn restore_from_cli() -> Result<(), String> {
    // No config = rshot never ran for this user (prerm runs this for everyone logged in,
    // gdm included): nothing to restore, and no config.toml to create.
    if !store::config_path().exists() {
        return Ok(());
    }
    restore_and_save(&mut store::load_config())
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

/// (shortcut, command) pairs to bind by hand on desktops rshot can't configure.
pub fn manual_commands(c: &Config) -> Vec<(String, String)> {
    let exe = exe_command();
    vec![
        (c.shortcuts.area.clone(), command_for(&exe, "capture area")),
        (
            c.shortcuts.screen.clone(),
            command_for(&exe, "capture screen"),
        ),
        (
            c.shortcuts.window.clone(),
            command_for(&exe, "capture window"),
        ),
    ]
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
