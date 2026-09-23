//! Config file, capture folders, file naming and atomic writes.

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
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
        Self {
            area: "Print".into(),
            screen: "Shift+Print".into(),
            window: "Alt+Print".into(),
        }
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
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rshot")
        .join("config.toml")
}

pub fn load_config() -> Config {
    load_config_from(&config_path())
}

/// A config that can't be read or parsed is moved to `config.toml.invalid`, so the next
/// save can't destroy what it held (e.g. the user's original GNOME keybindings).
pub fn load_config_from(p: &Path) -> Config {
    let err = match fs::read_to_string(p) {
        Ok(s) => match toml::from_str(&s) {
            Ok(c) => return c,
            Err(e) => e.to_string(),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Config::default(),
        Err(e) => e.to_string(),
    };
    let aside = p.with_extension("toml.invalid");
    eprintln!(
        "rshot: moving unusable {} to {}: {err}",
        p.display(),
        aside.display()
    );
    let _ = fs::rename(p, aside);
    Config::default()
}

pub fn save_config(c: &Config) -> io::Result<()> {
    save_config_to(&config_path(), c)
}

pub fn save_config_to(p: &Path, c: &Config) -> io::Result<()> {
    let s = toml::to_string(c).map_err(io::Error::other)?;
    write_atomic(p, s.as_bytes())
}

pub fn screenshots_dir(c: &Config) -> PathBuf {
    c.screenshots_dir
        .as_deref()
        .and_then(absolute)
        .unwrap_or_else(default_screenshots_dir)
}

fn default_screenshots_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Pictures")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Screenshots")
}

/// `~/x` → `<home>/x`; any other relative path → `None` (the clipboard only gets absolute paths).
fn absolute(p: &Path) -> Option<PathBuf> {
    match p.strip_prefix("~") {
        Ok(rest) => dirs::home_dir().map(|h| h.join(rest)),
        Err(_) => p.is_absolute().then(|| p.to_path_buf()),
    }
}

/// A folder picked in the UI; `None` means "the default folder".
pub fn dir_setting(chosen: &str) -> Option<PathBuf> {
    absolute(Path::new(chosen)).filter(|p| *p != default_screenshots_dir())
}

pub fn capture_stem(prefix: &str, t: NaiveDateTime) -> String {
    format!("{prefix}_{}", t.format("%Y-%m-%d_%H-%M-%S"))
}

/// Saves a capture as `Screenshot_<now>.png` in the screenshots folder; returns its path.
pub fn save_screenshot(c: &Config, png: &[u8]) -> io::Result<PathBuf> {
    let stem = capture_stem("Screenshot", Local::now().naive_local());
    write_new(&screenshots_dir(c), &stem, "png", png)
}

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// A hidden temp file in `dir`, unique per process and call (the daemon and the CLI both write
/// config.toml; two captures can share a name).
fn write_temp(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.{}-{n}.tmp", std::process::id()));
    fs::write(&tmp, bytes)?;
    Ok(tmp)
}

/// Writes to a temp file in the same folder, then renames, so readers never see half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("path has no file name"))?;
    let tmp = write_temp(dir, &name.to_string_lossy(), bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Saves `stem.ext`, or `stem_2.ext`, `stem_3.ext`… when taken, as complete files like
/// write_atomic, but never replaces one, even when another capture races for the same name.
pub fn write_new(dir: &Path, stem: &str, ext: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let tmp = write_temp(dir, stem, bytes)?;
    let placed = (1..)
        .map(|n| match n {
            1 => dir.join(format!("{stem}.{ext}")),
            n => dir.join(format!("{stem}_{n}.{ext}")),
        })
        .find_map(|p| match place_new(&tmp, &p) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => None,
            r => Some(r.map(|()| p)),
        })
        .expect("an unused name exists");
    let _ = fs::remove_file(&tmp);
    placed
}

/// Puts `tmp` at `p` unless something is already there (AlreadyExists).
fn place_new(tmp: &Path, p: &Path) -> io::Result<()> {
    match fs::hard_link(tmp, p) {
        // No hard links on this filesystem (FAT, some network shares): claim the name, then
        // replace the empty claim.
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)?;
            fs::rename(tmp, p).inspect_err(|_| {
                let _ = fs::remove_file(p);
            })
        }
        r => r,
    }
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
        let t = NaiveDate::from_ymd_opt(2026, 9, 22)
            .unwrap()
            .and_hms_opt(20, 41, 7)
            .unwrap();
        assert_eq!(
            capture_stem("Screenshot", t),
            "Screenshot_2026-09-22_20-41-07"
        );
    }

    #[test]
    fn write_new_appends_a_counter_and_keeps_existing_files() {
        let d = tmp("unique");
        assert_eq!(write_new(&d, "S", "png", b"1").unwrap(), d.join("S.png"));
        assert_eq!(write_new(&d, "S", "png", b"2").unwrap(), d.join("S_2.png"));
        assert_eq!(write_new(&d, "S", "png", b"3").unwrap(), d.join("S_3.png"));
        assert_eq!(fs::read(d.join("S.png")).unwrap(), b"1");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 3);
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
        c2.last_selection = Some(Selection {
            monitor: "DP-4".into(),
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        });
        c2.gnome_backup = Some([("screenshot".to_string(), "['<Shift>Print']".to_string())].into());
        c2.clipboard_mode = ClipboardMode::PathOnly;
        save_config_to(&p, &c2).unwrap();
        assert_eq!(load_config_from(&p), c2);
    }

    #[test]
    fn bad_config_is_moved_aside_not_lost() {
        let d = tmp("badconfig");
        let p = d.join("config.toml");
        let aside = d.join("config.toml.invalid");
        // Parse error, then a non-NotFound read error (invalid UTF-8).
        for bytes in [&b"this is = = not toml"[..], &b"\xff\xfe"[..]] {
            fs::write(&p, bytes).unwrap();
            assert_eq!(load_config_from(&p), Config::default());
            assert!(!p.exists());
            assert_eq!(fs::read(&aside).unwrap(), bytes);
        }
    }

    #[test]
    fn racing_captures_of_one_name_both_survive() {
        let d = tmp("race");
        for round in 0..50 {
            let stem = format!("S{round}");
            let barrier = std::sync::Barrier::new(2);
            let [a, b] = std::thread::scope(|s| {
                [&b"one"[..], &b"two"[..]]
                    .map(|bytes| {
                        let (d, stem, barrier) = (&d, &stem, &barrier);
                        s.spawn(move || {
                            barrier.wait();
                            write_new(d, stem, "png", bytes).unwrap()
                        })
                    })
                    .map(|h| h.join().unwrap())
            });
            assert_ne!(a, b);
            assert_eq!(fs::read(&a).unwrap(), b"one");
            assert_eq!(fs::read(&b).unwrap(), b"two");
        }
        assert_eq!(fs::read_dir(&d).unwrap().count(), 100, "no temp file left");
    }

    #[test]
    fn configured_folder_is_absolute_or_the_default() {
        let home = dirs::home_dir().unwrap();
        let abs = std::env::temp_dir().join("shots");
        let default = screenshots_dir(&Config::default());
        let with = |p: &str| {
            screenshots_dir(&Config {
                screenshots_dir: Some(p.into()),
                ..Config::default()
            })
        };
        assert_eq!(with("~/Shots"), home.join("Shots"));
        assert_eq!(with("Shots"), default);
        assert_eq!(with("./Shots"), default);
        assert_eq!(with(&abs.display().to_string()), abs);
        assert_eq!(dir_setting("~/Shots"), Some(home.join("Shots")));
        assert_eq!(dir_setting("Shots"), None);
        assert_eq!(dir_setting(&abs.display().to_string()), Some(abs));
    }

    #[test]
    fn dir_setting_maps_the_default_folder_to_none() {
        let default = screenshots_dir(&Config::default());
        assert_eq!(dir_setting(&default.display().to_string()), None);
        // Absolute on every platform ("/data/shots" isn't on Windows).
        let abs = std::env::temp_dir().join("shots");
        assert_eq!(dir_setting(&abs.display().to_string()), Some(abs));
    }
}
