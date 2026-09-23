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
#[serde(default)]
pub struct Shortcuts {
    pub area: String,
    pub screen: String,
    pub window: String,
    pub record: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            area: "Print".into(),
            screen: "Shift+Print".into(),
            window: "Alt+Print".into(),
            record: "Ctrl+Alt+Shift+R".into(),
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
    pub recordings_dir: Option<PathBuf>,
    /// PulseAudio/PipeWire source id; `None` records no sound.
    pub mic: Option<String>,
    pub fps: u8,
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
            recordings_dir: None,
            mic: None,
            fps: 30,
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

pub fn default_screenshots_dir() -> PathBuf {
    default_dir(dirs::picture_dir(), "Pictures", "Screenshots")
}

pub fn recordings_dir(c: &Config) -> PathBuf {
    c.recordings_dir
        .as_deref()
        .and_then(absolute)
        .unwrap_or_else(default_recordings_dir)
}

pub fn default_recordings_dir() -> PathBuf {
    default_dir(dirs::video_dir(), "Videos", "Screencasts")
}

/// `sub` in the XDG folder, else in `~/<home_sub>`.
fn default_dir(xdg: Option<PathBuf>, home_sub: &str, sub: &str) -> PathBuf {
    xdg.or_else(|| dirs::home_dir().map(|h| h.join(home_sub)))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(sub)
}

/// `~/x` → `<home>/x`; any other relative path → `None` (the clipboard only gets absolute paths).
fn absolute(p: &Path) -> Option<PathBuf> {
    match p.strip_prefix("~") {
        Ok(rest) => dirs::home_dir().map(|h| h.join(rest)),
        Err(_) => p.is_absolute().then(|| p.to_path_buf()),
    }
}

/// A folder picked in the UI; `None` means `default` (or a path that isn't absolute).
pub fn dir_setting(chosen: &str, default: &Path) -> Option<PathBuf> {
    absolute(Path::new(chosen)).filter(|p| p != default)
}

pub fn capture_stem(prefix: &str, t: NaiveDateTime) -> String {
    format!("{prefix}_{}", t.format("%Y-%m-%d_%H-%M-%S"))
}

/// Saves a capture as `Screenshot_<now>.png` in the screenshots folder; returns its path.
pub fn save_screenshot(c: &Config, png: &[u8]) -> io::Result<PathBuf> {
    let stem = capture_stem("Screenshot", Local::now().naive_local());
    write_new(&screenshots_dir(c), &stem, "png", png)
}

/// A free `Recording_<now>.mp4` (or `_2`, `_3`…) in the recordings folder, which is created.
pub fn new_recording_path(c: &Config) -> io::Result<PathBuf> {
    let dir = recordings_dir(c);
    fs::create_dir_all(&dir)?;
    let stem = capture_stem("Recording", Local::now().naive_local());
    Ok(free_recording(&dir, &stem))
}

/// The hidden `.<stem>.mkv` ffmpeg writes first; Stop remuxes it into `mp4`.
pub fn raw_recording(mp4: &Path) -> PathBuf {
    let stem = mp4.file_stem().unwrap_or_default().to_string_lossy();
    mp4.with_file_name(format!(".{stem}.mkv"))
}

/// A name is taken while its MP4 or its raw file exists (one of them does throughout a remux).
/// ponytail: an exists() check, not a claim; fine while starts are serialised (the recorder picks
/// the name under its lock). Claim with create_new if that ever changes.
fn free_recording(dir: &Path, stem: &str) -> PathBuf {
    numbered(dir, stem, "mp4")
        .find(|p| !p.exists() && !raw_recording(p).exists())
        .expect("an unused name exists")
}

/// `stem.ext`, else `stem_2.ext`, `stem_3.ext`…: the first that doesn't exist. (Unlike
/// `free_recording`, a raw file doesn't reserve the name: recovery turns that very file into it.)
pub fn unused(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    numbered(dir, stem, ext)
        .find(|p| !p.exists())
        .expect("an unused name exists")
}

/// A raw file no rshot finished (`.Recording_….mkv`: logout, crash, kill) → its recording's stem.
pub fn orphan_stem(name: &str) -> Option<&str> {
    name.strip_prefix('.')?
        .strip_suffix(".mkv")
        .filter(|s| s.starts_with("Recording_"))
}

/// A stop's or a trim's hidden temp output (`.….part.mp4`, `.….trim.mp4`).
pub fn stale_temp(name: &str) -> bool {
    name.starts_with('.') && (name.ends_with(".part.mp4") || name.ends_with(".trim.mp4"))
}

/// `stem.ext`, `stem_2.ext`, `stem_3.ext`…
fn numbered<'a>(dir: &'a Path, stem: &'a str, ext: &'a str) -> impl Iterator<Item = PathBuf> + 'a {
    (1..).map(move |n| match n {
        1 => dir.join(format!("{stem}.{ext}")),
        n => dir.join(format!("{stem}_{n}.{ext}")),
    })
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
    let placed = numbered(dir, stem, ext)
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
        let set = |p: &str| dir_setting(p, &default);
        assert_eq!(set("~/Shots"), Some(home.join("Shots")));
        assert_eq!(set("Shots"), None);
        assert_eq!(set(&abs.display().to_string()), Some(abs));
    }

    #[test]
    fn recording_defaults_and_old_configs_load() {
        let c = Config::default();
        assert_eq!(c.fps, 30);
        assert_eq!(c.mic, None);
        assert_eq!(c.shortcuts.record, "Ctrl+Alt+Shift+R");
        let d = tmp("oldcfg");
        let p = d.join("config.toml");
        fs::write(
            &p,
            "[shortcuts]\narea = \"F1\"\nscreen = \"Shift+Print\"\nwindow = \"Alt+Print\"\n",
        )
        .unwrap();
        // A config from before `record` existed keeps its values (it isn't moved aside as invalid).
        let old = load_config_from(&p).shortcuts;
        assert_eq!(
            (old.area.as_str(), old.record.as_str()),
            ("F1", "Ctrl+Alt+Shift+R")
        );
        fs::remove_dir_all(&d).unwrap();
        assert!(recordings_dir(&c).ends_with("Screencasts"));
    }

    #[test]
    fn recordings_folder_is_absolute_or_the_default() {
        let home = dirs::home_dir().unwrap();
        let default = recordings_dir(&Config::default());
        let with = |p: &str| {
            recordings_dir(&Config {
                recordings_dir: Some(p.into()),
                ..Config::default()
            })
        };
        assert_eq!(with("~/Casts"), home.join("Casts"));
        assert_eq!(with("Casts"), default);
        let abs = std::env::temp_dir().join("casts");
        assert_eq!(with(&abs.display().to_string()), abs);
    }

    #[test]
    fn recording_paths_are_new_and_count_up() {
        let d = tmp("recpath");
        let c = Config {
            recordings_dir: Some(d.join("sub")),
            ..Config::default()
        };
        let p = new_recording_path(&c).unwrap();
        assert_eq!(p.parent(), Some(d.join("sub").as_path()), "folder created");
        let name = p.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("Recording_") && name.ends_with(".mp4"),
            "{name}"
        );
        assert!(!p.exists(), "only named, not created");
        assert_eq!(
            raw_recording(&p),
            p.with_file_name(format!(".{}", name.replace(".mp4", ".mkv")))
        );
        assert_eq!(free_recording(&d, "R"), d.join("R.mp4"));
        fs::write(d.join("R.mp4"), b"").unwrap();
        assert_eq!(free_recording(&d, "R"), d.join("R_2.mp4"));
        // Still being remuxed (or kept after a crash): its raw file holds the name too.
        fs::write(d.join(".R_2.mkv"), b"").unwrap();
        assert_eq!(free_recording(&d, "R"), d.join("R_3.mp4"));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn leftovers_are_told_apart_by_name() {
        let stem = "Recording_2026-09-22_10-00-00";
        assert_eq!(orphan_stem(&format!(".{stem}.mkv")), Some(stem));
        assert_eq!(
            orphan_stem(&format!(".{stem}_2.mkv")),
            Some(&*format!("{stem}_2"))
        );
        assert_eq!(
            orphan_stem(&format!("{stem}.mkv")),
            None,
            "visible: the user's"
        );
        assert_eq!(orphan_stem(".Holiday.mkv"), None);
        assert_eq!(orphan_stem(&format!(".{stem}.part.mp4")), None);
        assert!(stale_temp(&format!(".{stem}.part.mp4")));
        assert!(stale_temp(".Holiday.trim.mp4"));
        assert!(!stale_temp(&format!("{stem}.mp4")));
        assert!(!stale_temp("Holiday.part.mp4"));
        assert!(!stale_temp(&format!(".{stem}.mkv")));
        // The orphan's own raw file doesn't keep it from its name.
        let d = tmp("unused");
        fs::write(d.join(".R.mkv"), b"").unwrap();
        assert_eq!(unused(&d, "R", "mp4"), d.join("R.mp4"));
        fs::write(d.join("R.mp4"), b"").unwrap();
        assert_eq!(unused(&d, "R", "mp4"), d.join("R_2.mp4"));
        assert_eq!(unused(&d, "R", "mkv"), d.join("R.mkv"));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn dir_setting_maps_the_default_folder_to_none() {
        let default = screenshots_dir(&Config::default());
        assert_eq!(default, default_screenshots_dir());
        assert_eq!(dir_setting(&default.display().to_string(), &default), None);
        // Absolute on every platform ("/data/shots" isn't on Windows).
        let abs = std::env::temp_dir().join("shots");
        assert_eq!(
            dir_setting(&abs.display().to_string(), &default),
            Some(abs.clone())
        );
        // The recordings default is the recordings folder's own, not the screenshots one.
        let rec = recordings_dir(&Config::default());
        assert_eq!(rec, default_recordings_dir());
        assert_eq!(dir_setting(&rec.display().to_string(), &rec), None);
        assert_eq!(
            dir_setting(&default.display().to_string(), &rec),
            Some(default)
        );
    }
}
