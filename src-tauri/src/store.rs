//! Config file, capture folders, file naming and atomic writes.

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ClipboardMode {
    #[default]
    PathAndImage,
    PathOnly,
}

/// Shortcuts in neutral "Mod+Mod+Key" form, e.g. "Shift+Print".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Shortcuts {
    pub area: String,
    pub screen: String,
    pub window: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self { area: "Print".into(), screen: "Shift+Print".into(), window: "Alt+Print".into() }
    }
}

/// Last area selection, in image pixels of the named monitor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Selection {
    pub monitor: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub screenshots_dir: Option<PathBuf>,
    pub clipboard_mode: ClipboardMode,
    pub show_thumbnail: bool,
    pub shutter_sound: bool,
    pub remember_selection: bool,
    pub show_pointer: bool,
    pub timer_secs: u8,
    pub last_selection: Option<Selection>,
    pub hints_shown: u32,
    pub onboarded: bool,
    pub takeover: bool,
    pub shortcuts: Shortcuts,
    /// Original GNOME keybinding values (GVariant text), kept while rshot owns them.
    pub gnome_backup: Option<BTreeMap<String, String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            screenshots_dir: None,
            clipboard_mode: ClipboardMode::PathAndImage,
            show_thumbnail: true,
            shutter_sound: true,
            remember_selection: true,
            show_pointer: false,
            timer_secs: 0,
            last_selection: None,
            hints_shown: 0,
            onboarded: false,
            takeover: false,
            shortcuts: Shortcuts::default(),
            gnome_backup: None,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("rshot").join("config.toml")
}

pub fn load_config() -> Config {
    load_config_from(&config_path())
}

pub fn load_config_from(p: &Path) -> Config {
    match fs::read_to_string(p) {
        Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
            eprintln!("rshot: ignoring invalid {}: {e}", p.display());
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

pub fn save_config(c: &Config) -> io::Result<()> {
    save_config_to(&config_path(), c)
}

pub fn save_config_to(p: &Path, c: &Config) -> io::Result<()> {
    let s = toml::to_string(c).map_err(io::Error::other)?;
    write_atomic(p, s.as_bytes())
}

pub fn screenshots_dir(c: &Config) -> PathBuf {
    c.screenshots_dir.clone().unwrap_or_else(|| {
        dirs::picture_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Pictures")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Screenshots")
    })
}

/// A folder picked in the UI; `None` means "the default folder".
pub fn dir_setting(chosen: &str) -> Option<PathBuf> {
    let p = PathBuf::from(chosen);
    (p != screenshots_dir(&Config::default())).then_some(p)
}

pub fn capture_stem(prefix: &str, t: NaiveDateTime) -> String {
    format!("{prefix}_{}", t.format("%Y-%m-%d_%H-%M-%S"))
}

pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}_{n}.{ext}")))
        .find(|p| !p.exists())
        .expect("an unused name exists")
}

pub fn new_screenshot_path(c: &Config) -> io::Result<PathBuf> {
    let dir = screenshots_dir(c);
    fs::create_dir_all(&dir)?;
    Ok(unique_path(&dir, &capture_stem("Screenshot", Local::now().naive_local()), "png"))
}

/// Writes to a temp file in the same folder, then renames, so readers never see half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| io::Error::other("path has no parent"))?;
    fs::create_dir_all(dir)?;
    let name = path.file_name().ok_or_else(|| io::Error::other("path has no file name"))?;
    let tmp = dir.join(format!(".{}.tmp", name.to_string_lossy()));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rshot-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn stem_has_no_spaces() {
        let t = NaiveDate::from_ymd_opt(2026, 9, 22).unwrap().and_hms_opt(20, 41, 7).unwrap();
        assert_eq!(capture_stem("Screenshot", t), "Screenshot_2026-09-22_20-41-07");
    }

    #[test]
    fn unique_path_appends_a_counter() {
        let d = tmp("unique");
        assert_eq!(unique_path(&d, "S", "png"), d.join("S.png"));
        fs::write(d.join("S.png"), b"").unwrap();
        assert_eq!(unique_path(&d, "S", "png"), d.join("S_2.png"));
        fs::write(d.join("S_2.png"), b"").unwrap();
        assert_eq!(unique_path(&d, "S", "png"), d.join("S_3.png"));
    }

    #[test]
    fn write_atomic_creates_dirs_and_leaves_no_temp_file() {
        let d = tmp("atomic");
        let p = d.join("sub").join("out.png");
        write_atomic(&p, b"abc").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"abc");
        assert_eq!(fs::read_dir(p.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn config_round_trips_and_fills_defaults() {
        let d = tmp("config");
        let p = d.join("config.toml");
        assert_eq!(load_config_from(&p), Config::default());
        fs::write(&p, "show_thumbnail = false\n").unwrap();
        let c = load_config_from(&p);
        assert!(!c.show_thumbnail);
        assert!(c.shutter_sound);
        let mut c2 = c.clone();
        c2.last_selection = Some(Selection { monitor: "DP-4".into(), x: 1, y: 2, w: 3, h: 4 });
        c2.gnome_backup = Some([("screenshot".to_string(), "['<Shift>Print']".to_string())].into());
        c2.clipboard_mode = ClipboardMode::PathOnly;
        save_config_to(&p, &c2).unwrap();
        assert_eq!(load_config_from(&p), c2);
        fs::write(&p, "this is = = not toml").unwrap();
        assert_eq!(load_config_from(&p), Config::default());
    }

    #[test]
    fn dir_setting_maps_the_default_folder_to_none() {
        let default = screenshots_dir(&Config::default());
        assert_eq!(dir_setting(&default.display().to_string()), None);
        assert_eq!(dir_setting("/data/shots"), Some(PathBuf::from("/data/shots")));
    }
}
