//! GNOME: clear the Shell screenshot keys and add custom keybindings that run rshot.

use super::gvariant::{format_strv, parse_strv, quote, to_gnome_accel, with_paths, without_paths};
use crate::store::Config;
use std::{collections::BTreeMap, process::Command};

const SHELL: &str = "org.gnome.shell.keybindings";
/// Plan 3 adds "show-screen-recording-ui".
const TAKEN_KEYS: [&str; 3] = ["show-screenshot-ui", "screenshot", "screenshot-window"];
const MEDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const BASE: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";
const OURS: [(&str, &str); 3] = [("area", "capture area"), ("screen", "capture screen"), ("window", "capture window")];

fn gs(args: &[&str]) -> Result<String, String> {
    let out = Command::new("gsettings").args(args).output().map_err(|e| format!("gsettings: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!("gsettings {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
    }
}

fn path(id: &str) -> String {
    format!("{BASE}/rshot-{id}/")
}

fn our_paths() -> Vec<String> {
    OURS.iter().map(|(id, _)| path(id)).collect()
}

pub fn take_over(cfg: &mut Config, exe: &str) -> Result<(), String> {
    if cfg.gnome_backup.is_none() {
        let mut backup = BTreeMap::new();
        for k in TAKEN_KEYS {
            backup.insert(k.to_string(), gs(&["get", SHELL, k])?);
        }
        cfg.gnome_backup = Some(backup);
    }
    for k in TAKEN_KEYS {
        gs(&["set", SHELL, k, "@as []"])?;
    }
    for (id, sub) in OURS {
        let schema = format!("{CUSTOM}:{}", path(id));
        let accel = match id {
            "area" => &cfg.shortcuts.area,
            "screen" => &cfg.shortcuts.screen,
            _ => &cfg.shortcuts.window,
        };
        gs(&["set", &schema, "name", &quote(&format!("rshot {id}"))])?;
        gs(&["set", &schema, "command", &quote(&format!("\"{exe}\" {sub}"))])?;
        gs(&["set", &schema, "binding", &quote(&to_gnome_accel(accel))])?;
    }
    let list = parse_strv(&gs(&["get", MEDIA, "custom-keybindings"])?);
    gs(&["set", MEDIA, "custom-keybindings", &format_strv(&with_paths(&list, &our_paths()))])?;
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
    gs(&["set", MEDIA, "custom-keybindings", &format_strv(&without_paths(&list, &our_paths()))])?;
    for (id, _) in OURS {
        let _ = gs(&["reset-recursively", &format!("{CUSTOM}:{}", path(id))]);
    }
    Ok(())
}
