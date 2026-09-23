//! GNOME: clear the Shell screenshot and screen-recording keys and add custom keybindings that
//! run rshot.

use super::gvariant::{format_strv, parse_strv, quote, to_gnome_accel, with_paths, without_paths};
use crate::store::{self, Config};
use std::{collections::BTreeMap, process::Command};

const SHELL: &str = "org.gnome.shell.keybindings";
/// A takeover saved by an older rshot may lack keys added since: see `outdated`.
const TAKEN_KEYS: [&str; 4] = [
    "show-screenshot-ui",
    "screenshot",
    "screenshot-window",
    "show-screen-recording-ui",
];
const MEDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const BASE: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";
const OURS: [(&str, &str); 4] = [
    ("area", "capture area"),
    ("screen", "capture screen"),
    ("window", "capture window"),
    ("record", "record"),
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

/// Taken over, but by a version that didn't take every key this one does (e.g. the record key):
/// `take_over` again takes the rest, backing up their originals first.
pub fn outdated(cfg: &Config) -> bool {
    let backup = cfg.gnome_backup.as_ref();
    cfg.takeover
        && TAKEN_KEYS
            .iter()
            .any(|k| !backup.is_some_and(|b| b.contains_key(*k)))
}

pub fn take_over(cfg: &mut Config, exe: &str) -> Result<(), String> {
    // Validate before touching anything.
    let s = &cfg.shortcuts;
    let accels = [
        to_gnome_accel(&s.area)?,
        to_gnome_accel(&s.screen)?,
        to_gnome_accel(&s.window)?,
        to_gnome_accel(&s.record)?,
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

#[derive(Debug, PartialEq)]
enum Put<'a> {
    Set(&'a str),
    Reset,
}

/// How one Shell key goes back. An empty list is never a real original: it is rshot's own cleared
/// value, backed up as the "original" when config.toml was lost while the keys were taken. Setting
/// it would leave Print dead for good; a reset gives GNOME's default back.
fn put_back(v: &str) -> Put<'_> {
    if parse_strv(v).is_empty() {
        Put::Reset
    } else {
        Put::Set(v)
    }
}

/// Whether a key read back after a reset still needs its original set (GVariant text compared as
/// lists, so quoting and spacing don't matter).
fn differs(after_reset: &str, original: &str) -> bool {
    parse_strv(after_reset) != parse_strv(original)
}

pub fn restore(cfg: &mut Config) -> Result<(), String> {
    let backup = cfg.gnome_backup.clone().unwrap_or_default();
    for (k, v) in &backup {
        match put_back(v) {
            // Through the default first: an original equal to it (the usual case) then leaves no
            // user value behind in dconf.
            Put::Set(v) => {
                gs(&["reset", SHELL, k])?;
                if differs(&gs(&["get", SHELL, k])?, v) {
                    gs(&["set", SHELL, k, v])?;
                }
            }
            Put::Reset => {
                gs(&["reset", SHELL, k])?;
            }
        }
    }
    // A taken key with no backup (config.toml lost, then "Not now") may still hold rshot's `[]`.
    for k in TAKEN_KEYS.iter().filter(|k| !backup.contains_key(**k)) {
        if put_back(&gs(&["get", SHELL, k])?) == Put::Reset {
            gs(&["reset", SHELL, k])?;
        }
    }
    cfg.gnome_backup = None;
    let list = parse_strv(&gs(&["get", MEDIA, "custom-keybindings"])?);
    let rest = without_paths(&list, &our_paths());
    if rest.is_empty() {
        gs(&["reset", MEDIA, "custom-keybindings"])?;
    } else {
        gs(&["set", MEDIA, "custom-keybindings", &format_strv(&rest)])?;
    }
    for (id, _) in OURS {
        let _ = gs(&["reset-recursively", &format!("{CUSTOM}:{}", path(id))]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{differs, gsettings_ok, outdated, put_back, Put, TAKEN_KEYS};
    use crate::store::Config;

    #[test]
    fn a_takeover_missing_a_key_is_outdated() {
        let with = |takeover: bool, keys: &[&str]| Config {
            takeover,
            gnome_backup: Some(
                keys.iter()
                    .map(|k| (k.to_string(), "['x']".into()))
                    .collect(),
            ),
            ..Config::default()
        };
        // Plan 1's takeover backed up three keys; the record key is missing.
        assert!(outdated(&with(true, &TAKEN_KEYS[..3])));
        assert!(!outdated(&with(true, &TAKEN_KEYS)));
        assert!(!outdated(&with(false, &TAKEN_KEYS[..3])));
        assert!(outdated(&Config {
            takeover: true,
            ..Config::default()
        }));
        assert!(!outdated(&Config::default()));
    }

    #[test]
    fn an_original_equal_to_the_default_is_not_set() {
        assert!(!differs("['Print']", "['Print']"));
        assert!(!differs(
            "['<Ctrl><Shift><Alt>R']",
            "[\"<Ctrl><Shift><Alt>R\"]"
        ));
        assert!(differs("['Print']", "['<Super>Print']"));
        assert!(differs("['Print']", "['Print', 'F12']"));
    }

    #[test]
    fn an_empty_original_is_reset_not_set() {
        assert_eq!(put_back("['Print']"), Put::Set("['Print']"));
        assert_eq!(put_back("@as []"), Put::Reset);
        assert_eq!(put_back("[]"), Put::Reset);
    }

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
