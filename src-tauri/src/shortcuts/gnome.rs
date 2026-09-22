//! GNOME: clear the Shell screenshot keys and add custom keybindings that run rshot.

use super::gvariant::{format_strv, parse_strv, quote, to_gnome_accel, with_paths, without_paths};
use crate::store::{self, Config};
use std::{collections::BTreeMap, process::Command};

const SHELL: &str = "org.gnome.shell.keybindings";
/// Plan 3 adds "show-screen-recording-ui".
const TAKEN_KEYS: [&str; 3] = ["show-screenshot-ui", "screenshot", "screenshot-window"];
const MEDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const BASE: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";
const OURS: [(&str, &str); 3] = [
    ("area", "capture area"),
    ("screen", "capture screen"),
    ("window", "capture window"),
];

/// `gsettings set` exits 0 even when dconf can't commit (e.g. no session bus): it only warns on
/// stderr. A call that worked prints nothing there.
fn gsettings_ok(status_ok: bool, stderr: &[u8]) -> bool {
    status_ok && stderr.is_empty()
}

fn gs(args: &[&str]) -> Result<String, String> {
    let out = Command::new("gsettings")
        .args(args)
        .output()
        .map_err(|e| format!("gsettings: {e}"))?;
    if gsettings_ok(out.status.success(), &out.stderr) {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "gsettings {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn path(id: &str) -> String {
    format!("{BASE}/rshot-{id}/")
}

fn our_paths() -> Vec<String> {
    OURS.iter().map(|(id, _)| path(id)).collect()
}

pub fn take_over(cfg: &mut Config, exe: &str) -> Result<(), String> {
    // Validate before touching anything.
    let s = &cfg.shortcuts;
    let accels = [
        to_gnome_accel(&s.area)?,
        to_gnome_accel(&s.screen)?,
        to_gnome_accel(&s.window)?,
    ];
    // Per key, so keys added in later versions get backed up too; never overwrite an original.
    let backup = cfg.gnome_backup.get_or_insert_with(BTreeMap::new);
    for k in TAKEN_KEYS {
        if !backup.contains_key(k) {
            backup.insert(k.to_string(), gs(&["get", SHELL, k])?);
        }
    }
    // The originals reach disk before the first change, so a crash mid-way can still restore.
    store::save_config(cfg).map_err(|e| format!("saving shortcut backup: {e}"))?;
    for k in TAKEN_KEYS {
        gs(&["set", SHELL, k, "@as []"])?;
    }
    for ((id, sub), accel) in OURS.iter().zip(&accels) {
        let schema = format!("{CUSTOM}:{}", path(id));
        gs(&["set", &schema, "name", &quote(&format!("rshot {id}"))])?;
        gs(&[
            "set",
            &schema,
            "command",
            &quote(&super::command_for(exe, sub)),
        ])?;
        gs(&["set", &schema, "binding", &quote(accel)])?;
    }
    let list = parse_strv(&gs(&["get", MEDIA, "custom-keybindings"])?);
    gs(&[
        "set",
        MEDIA,
        "custom-keybindings",
        &format_strv(&with_paths(&list, &our_paths())),
    ])?;
    Ok(())
}

pub fn restore(cfg: &mut Config) -> Result<(), String> {
    if let Some(backup) = &cfg.gnome_backup {
        for (k, v) in backup {
            gs(&["set", SHELL, k, v])?;
        }
    }
    cfg.gnome_backup = None;
    let list = parse_strv(&gs(&["get", MEDIA, "custom-keybindings"])?);
    gs(&[
        "set",
        MEDIA,
        "custom-keybindings",
        &format_strv(&without_paths(&list, &our_paths())),
    ])?;
    for (id, _) in OURS {
        let _ = gs(&["reset-recursively", &format!("{CUSTOM}:{}", path(id))]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::gsettings_ok;

    #[test]
    fn a_dconf_warning_is_a_failure() {
        assert!(gsettings_ok(true, b""));
        assert!(!gsettings_ok(
            true,
            b"(process:1): dconf-WARNING **: failed to commit changes to dconf\n"
        ));
        assert!(!gsettings_ok(false, b""));
    }
}
