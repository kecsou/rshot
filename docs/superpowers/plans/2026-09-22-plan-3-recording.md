# rshot Plan 3: Screen Recording + Video Trim — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record an area or the full screen to MP4 (H.264, optional mic). `Ctrl+Alt+Shift+R` or the overlay's record modes start it, after a 3-second countdown. A dashed frame and a control pill sit outside the area, and the tray shows a timer. Stop gives a video thumbnail with the path copied. A trim editor saves in place. ffmpeg ships **inside** the `.deb`.

**Architecture:**
- `recorder.rs` holds pure argument builders (unit-tested) and process control for a bundled static ffmpeg sidecar (`rshot-ffmpeg`).
- ffmpeg writes a crash-safe MKV next to the target. On stop it gets `q`, and the MKV is remuxed to MP4 (`-c copy`).
- The overlay gains the two record modes. Recording windows (frame + pill) are small transparent popups outside the recorded area.
- Videos reach webviews through Tauri's asset protocol (streamed with range requests, never read whole through IPC).
- The video editor reuses Plan 2's per-file window map. `trim_video` re-encodes the kept range to a temp file and renames it over the original.

**Tech Stack:** Plans 1–2, plus a static ffmpeg 8.1 (BtbN FFmpeg-Builds, GPL) as a Tauri `externalBin`, and Tauri's `protocol-asset` feature.

**Spec:** `docs/superpowers/specs/2026-09-22-rshot-design.md` §2.1 (record shortcut), §2.6, §2.7, §2.8 (recording settings), §3.1 (ffmpeg sidecar), §4 (ffmpeg rows). Visual contract: `docs/design/mockups/04-recording.png`, and 02c/05c for the mic and fps controls.

## Global Constraints

- Plan 1's Global Constraints and Environment notes apply (`docs/superpowers/plans/2026-09-22-plan-1-foundation-linux-screenshots.md`).
- Video file name: `<XDG Videos>/Screencasts/Recording_YYYY-MM-DD_HH-MM-SS.mp4`. The folder is configurable.
- Encoding: H.264 `libx264 -preset veryfast -crf 23 -pix_fmt yuv420p`, plus AAC 160k when a mic is on. Recorded area dimensions are rounded down to even.
- Temp capture: `.<stem>.mkv` in the same folder. On stop, remux with `-c copy -movflags +faststart`, then delete the MKV. If ffmpeg dies, keep the MKV and notify its path.
- Countdown: 3 s before recording starts. `Esc` or a click cancels.
- Frame: a dashed red (`#ff453a`) border drawn **outside** the area, click-through. The pill sits outside the area too.
- Full-screen recordings: no frame or pill; the tray shows the timer.
- Stop by: the pill, the tray item "Stop Recording", or the record shortcut (`rshot record` again).
- Clipboard for videos: the path text + file formats, never an image (Path-only mode gives just the text).
- Settings: microphone (None or a PulseAudio/PipeWire source), frame rate 30/60, recordings folder, and a "Record screen" shortcut (default `Ctrl+Alt+Shift+R`, taken over from GNOME `show-screen-recording-ui`).
- Sidecar name: `rshot-ffmpeg`, never `ffmpeg`. The `.deb` installs it at `/usr/bin/rshot-ffmpeg` without clashing with the distro `ffmpeg`. It's fetched by `scripts/fetch-ffmpeg.sh`, never committed. `THIRD_PARTY.md` carries the GPL notice and source link.
- The mic level meter from mockup 4a is **not** built. The pill shows a mic icon when the mic is on.

## File Map

```
scripts/fetch-ffmpeg.sh                 # downloads the sidecar into src-tauri/binaries/
src-tauri/tauri.linux.conf.json         # externalBin (Linux only until Plan 4)
src-tauri/src/recorder.rs               # args (pure, tested), mic list, start/stop/discard/watchdog, trim
src-tauri/src/store.rs                  # + recordings_dir, mic, fps, shortcuts.record, new_recording_path
src-tauri/src/pipeline.rs               # + finish_video
src-tauri/src/overlay.rs                # + overlay_record, record countdown
src-tauri/src/ui.rs                     # + recording windows, tray record item/timer, video editor page
src-tauri/src/thumbnail.rs              # Thumb.kind
src-tauri/src/editor.rs                 # + trim_video
src-tauri/src/settings.rs               # + recording settings, record shortcut
src-tauri/src/shortcuts/gnome.rs, mod.rs # + record binding
src-tauri/src/cli.rs, main.rs           # + `record`
src/recframe/index.html                 # dashed frame
src/pill/{index.html,main.ts}           # recording pill
src/video/{index.html,video.css,main.ts,trim.ts,trim.test.ts}
src/overlay/{index.html,main.ts,options.ts}  # record modes, mic picker
src/thumbnail/main.ts, src/settings/*   # video thumbnail; recording group
THIRD_PARTY.md
```

---

### Task 1: ffmpeg sidecar + pure recorder functions

**Files:**
- Create: `scripts/fetch-ffmpeg.sh`, `src-tauri/tauri.linux.conf.json`, `src-tauri/src/recorder.rs`, `THIRD_PARTY.md`
- Modify: `.gitignore`, `src-tauri/src/main.rs` (`mod recorder;`), `.github/workflows/ci.yml` (fetch step)
- Test: unit tests in `recorder.rs`

**Interfaces:**
- Produces:
  - `recorder::Region { x: i32, y: i32, w: u32, h: u32 }` (Serialize, Copy) and `recorder::Mic { id, label }` (Serialize).
  - Pure functions:

    ```rust
    even(u32) -> u32
    parse_pulse_sources(&str) -> Vec<Mic>
    record_args(Region, display: &str, fps: u8, mic: Option<&str>, out: &Path) -> Result<Vec<String>, String>  // Err off Linux until Plan 4
    remux_args(&Path, &Path) -> Vec<String>
    trim_args(src: &Path, start: f64, end: f64, mute: bool, out: &Path) -> Vec<String>
    ```

  - `recorder::ffmpeg_path() -> Option<PathBuf>`: the sidecar next to the exe, else `ffmpeg` on `PATH`.

- [ ] **Step 1: Write the fetch script**

`scripts/fetch-ffmpeg.sh`:

```sh
#!/bin/sh
# Fetches the static ffmpeg rshot bundles as a Tauri sidecar:
#   src-tauri/binaries/rshot-ffmpeg-<rust target triple>
# Linux x86_64: BtbN FFmpeg-Builds, 8.1 release branch, GPL static (x11grab, pulse, libx264, aac).
set -eu
cd "$(dirname "$0")/../src-tauri"
mkdir -p binaries
triple=$(rustc -vV | sed -n 's/^host: //p')
out="binaries/rshot-ffmpeg-$triple"
if [ -x "$out" ]; then echo "$out already present"; exit 0; fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
case "$triple" in
  x86_64-unknown-linux-gnu)
    curl -fsSL https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.1-latest-linux64-gpl-8.1.tar.xz | tar -xJ -C "$tmp"
    cp "$tmp"/ffmpeg-*/bin/ffmpeg "$out"
    ;;
  *)
    echo "no bundled ffmpeg for $triple yet" >&2
    exit 1
    ;;
esac
chmod +x "$out"
"$out" -hide_banner -devices 2>/dev/null | grep -q x11grab || { echo "sidecar lacks x11grab" >&2; exit 1; }
"$out" -hide_banner -encoders 2>/dev/null | grep -q libx264 || { echo "sidecar lacks libx264" >&2; exit 1; }
echo "fetched $out ($("$out" -version | head -1))"
```

Run: `chmod +x scripts/fetch-ffmpeg.sh && scripts/fetch-ffmpeg.sh`
Expected: `fetched binaries/rshot-ffmpeg-x86_64-unknown-linux-gnu (ffmpeg version n8.1…)`.

Append `src-tauri/binaries/` to `.gitignore`.

- [ ] **Step 2: Declare the sidecar for Linux builds only**

`src-tauri/tauri.linux.conf.json` (Tauri merges it over `tauri.conf.json` on Linux):

```json
{
  "bundle": {
    "externalBin": ["binaries/rshot-ffmpeg"]
  }
}
```

In `.github/workflows/ci.yml`, add this step to the Linux job right after "Linux system libraries":

```yaml
      - name: Bundled ffmpeg sidecar
        if: runner.os == 'Linux'
        run: scripts/fetch-ffmpeg.sh
```

`THIRD_PARTY.md`:

```markdown
# Third-party software

## FFmpeg (bundled as `rshot-ffmpeg`)

rshot's installers include an unmodified static build of FFmpeg, used as a separate program for screen recording
and trimming. FFmpeg is licensed under the GNU GPL v3 (this build enables GPL components such as libx264).

- Linux build: BtbN FFmpeg-Builds, release branch 8.1 — https://github.com/BtbN/FFmpeg-Builds
- FFmpeg source: https://ffmpeg.org/download.html (tag `n8.1.x`), build scripts: https://github.com/BtbN/FFmpeg-Builds
```

- [ ] **Step 3: Write `recorder.rs` with stubs and failing tests**

```rust
//! Screen recording through the bundled static ffmpeg (`rshot-ffmpeg`).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// A region in physical desktop pixels.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Mic {
    pub id: String,
    pub label: String,
}

/// The sidecar next to our executable (installed or dev), else `ffmpeg` on PATH.
pub fn ffmpeg_path() -> Option<PathBuf> {
    let name = if cfg!(windows) { "rshot-ffmpeg.exe" } else { "rshot-ffmpeg" };
    let bundled = std::env::current_exe().ok()?.parent()?.join(name);
    if bundled.is_file() {
        return Some(bundled);
    }
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(exe)).find(|p| p.is_file())
}

/// yuv420p needs even dimensions.
pub fn even(v: u32) -> u32 {
    v
}

/// Parses `ffmpeg -sources pulse`, keeping inputs (monitor sources are dropped).
pub fn parse_pulse_sources(_out: &str) -> Vec<Mic> {
    Vec::new()
}

pub fn record_args(_r: Region, _display: &str, _fps: u8, _mic: Option<&str>, _out: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

pub fn remux_args(_mkv: &Path, _mp4: &Path) -> Vec<String> {
    Vec::new()
}

pub fn trim_args(_src: &Path, _start: f64, _end: f64, _mute: bool, _out: &Path) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn even_rounds_down() {
        assert_eq!(even(871), 870);
        assert_eq!(even(870), 870);
    }

    #[test]
    fn parses_pulse_inputs_only() {
        let out = "Auto-detected sources for pulse:\n  alsa_output.x.monitor [Monitor of X] (none)\n* alsa_input.usb-mic.mono [USB Mic] (none)\n  alsa_input.pci.analog-stereo [Built-in Audio] (none)\n";
        assert_eq!(
            parse_pulse_sources(out),
            vec![
                Mic { id: "alsa_input.usb-mic.mono".into(), label: "USB Mic".into() },
                Mic { id: "alsa_input.pci.analog-stereo".into(), label: "Built-in Audio".into() },
            ]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn record_args_grab_x11_with_optional_mic() {
        let r = Region { x: 1920, y: 10, w: 871, h: 569 };
        let a = record_args(r, ":1", 30, None, Path::new("/v/.R.mkv")).unwrap();
        assert_eq!(
            a,
            s(&[
                "-hide_banner", "-loglevel", "error", "-f", "x11grab", "-framerate", "30", "-draw_mouse", "1",
                "-video_size", "870x568", "-i", ":1+1920,10", "-c:v", "libx264", "-preset", "veryfast", "-crf", "23",
                "-pix_fmt", "yuv420p", "-y", "/v/.R.mkv",
            ])
        );
        let m = record_args(r, ":1", 60, Some("alsa_input.usb"), Path::new("/v/.R.mkv")).unwrap();
        assert!(m.windows(4).any(|w| w == s(&["-f", "pulse", "-i", "alsa_input.usb"])));
        assert!(m.windows(4).any(|w| w == s(&["-c:a", "aac", "-b:a", "160k"])));
        assert!(m.windows(2).any(|w| w == s(&["-framerate", "60"])));
    }

    #[test]
    fn remux_copies_streams() {
        assert_eq!(
            remux_args(Path::new("/v/.R.mkv"), Path::new("/v/R.mp4")),
            s(&["-hide_banner", "-loglevel", "error", "-i", "/v/.R.mkv", "-c", "copy", "-movflags", "+faststart", "-y", "/v/R.mp4"])
        );
    }

    #[test]
    fn trim_reencodes_the_kept_range() {
        let a = trim_args(Path::new("/v/R.mp4"), 4.8, 28.6, true, Path::new("/v/.R.trim.mp4"));
        assert_eq!(
            a,
            s(&[
                "-hide_banner", "-loglevel", "error", "-ss", "4.800", "-i", "/v/R.mp4", "-t", "23.800", "-c:v", "libx264",
                "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p", "-an", "-movflags", "+faststart", "-y",
                "/v/.R.trim.mp4",
            ])
        );
        let b = trim_args(Path::new("/v/R.mp4"), 0.0, 1.0, false, Path::new("/o.mp4"));
        assert!(b.windows(2).any(|w| w == s(&["-c:a", "aac"])));
    }
}
```

In `main.rs`, add `mod recorder;`.

Run: `cargo test recorder`
Expected: FAIL. `even_rounds_down`, `parses_pulse_inputs_only`, `record_args_…`, `remux_…` and `trim_…` all fail.

- [ ] **Step 4: Implement the pure functions**

```rust
pub fn even(v: u32) -> u32 {
    v & !1
}

pub fn parse_pulse_sources(out: &str) -> Vec<Mic> {
    out.lines()
        .filter_map(|line| {
            let line = line.trim_start_matches(['*', ' ']);
            let (id, rest) = line.split_once(' ')?;
            let label = rest.split_once('[')?.1.split_once(']')?.0;
            (!id.ends_with(".monitor")).then(|| Mic { id: id.into(), label: label.into() })
        })
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[cfg(target_os = "linux")]
pub fn record_args(r: Region, display: &str, fps: u8, mic: Option<&str>, out: &Path) -> Result<Vec<String>, String> {
    let mut a = strs(&["-hide_banner", "-loglevel", "error", "-f", "x11grab", "-framerate"]);
    a.push(fps.to_string());
    a.extend(strs(&["-draw_mouse", "1", "-video_size"]));
    a.push(format!("{}x{}", even(r.w), even(r.h)));
    a.push("-i".into());
    a.push(format!("{display}+{},{}", r.x, r.y));
    if let Some(m) = mic {
        a.extend(strs(&["-f", "pulse", "-i", m]));
    }
    a.extend(strs(&["-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p"]));
    if mic.is_some() {
        a.extend(strs(&["-c:a", "aac", "-b:a", "160k"]));
    }
    a.push("-y".into());
    a.push(out.display().to_string());
    Ok(a)
}

#[cfg(not(target_os = "linux"))]
pub fn record_args(_r: Region, _display: &str, _fps: u8, _mic: Option<&str>, _out: &Path) -> Result<Vec<String>, String> {
    Err("Recording on this OS arrives in a later version.".into())
}

pub fn remux_args(mkv: &Path, mp4: &Path) -> Vec<String> {
    let (mkv, mp4) = (mkv.display().to_string(), mp4.display().to_string());
    strs(&["-hide_banner", "-loglevel", "error", "-i", &mkv, "-c", "copy", "-movflags", "+faststart", "-y", &mp4])
}

/// Frame-accurate: `-ss` before `-i` with a re-encode seeks exactly.
pub fn trim_args(src: &Path, start: f64, end: f64, mute: bool, out: &Path) -> Vec<String> {
    let (src, out) = (src.display().to_string(), out.display().to_string());
    let (ss, t) = (format!("{start:.3}"), format!("{:.3}", end - start));
    let mut a = strs(&["-hide_banner", "-loglevel", "error", "-ss", &ss, "-i", &src, "-t", &t]);
    a.extend(strs(&["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"]));
    a.extend(if mute { strs(&["-an"]) } else { strs(&["-c:a", "aac"]) });
    a.extend(strs(&["-movflags", "+faststart", "-y", &out]));
    a
}
```

Run: `cargo test recorder`
Expected: `5 passed`.

- [ ] **Step 5: Commit**

```bash
git add scripts/fetch-ffmpeg.sh src-tauri/tauri.linux.conf.json src-tauri/src/recorder.rs src-tauri/src/main.rs .gitignore THIRD_PARTY.md .github/workflows/ci.yml
git commit -m "feat(record): bundled ffmpeg sidecar and tested ffmpeg argument builders

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Recording engine — config, start/stop/discard, watchdog, tray, `rshot record`

**Files:**
- Modify:
  - `src-tauri/src/store.rs` (fields, `new_recording_path`, `recordings_dir`), `src-tauri/src/recorder.rs` (process control, `list_mics`), `src-tauri/src/pipeline.rs` (`finish_video`), `src-tauri/src/thumbnail.rs` (`Thumb.kind`)
  - `src-tauri/src/cli.rs` (`record`), `src-tauri/src/main.rs`, `src-tauri/src/ui.rs` (tray Record item + timer, recording window hooks as no-ops until Task 3)
  - `src/shared/ipc.ts` (`Thumb.kind`, `listMics`)
- Test: unit tests in `store.rs` and `cli.rs`; an `#[ignore]` integration test in `recorder.rs` that really records 1 s

**Interfaces:**
- Consumes: Task 1 functions, `AppState`, `pipeline::notify`, `ui::{show_thumbnail, close_prefix}`.
- Produces:
  - Config fields: `recordings_dir: Option<PathBuf>`, `mic: Option<String>`, `fps: u8` (default 30), `shortcuts.record` (default `"Ctrl+Alt+Shift+R"`, `#[serde(default)]` on `Shortcuts`).
  - `store::{recordings_dir(&Config) -> PathBuf, new_recording_path(&Config) -> io::Result<PathBuf>}`.
  - Recorder API:

    ```rust
    recorder::start(&AppHandle, Region, full: bool) -> Result<(), String>
    recorder::stop(&AppHandle) -> Result<(), String>
    recorder::discard(&AppHandle)
    recorder::is_recording(&AppHandle) -> bool
    ```

  - Commands: `recording_stop`, `recording_discard`, `recording_info -> { elapsed_ms: u64, mic: bool }`, `list_mics -> Vec<Mic>`.
  - `pipeline::finish_video(&AppHandle, &Path) -> Result<(), String>`, `thumbnail::Thumb.kind: String` (`"image"`/`"video"`) and `cli::Cmd::Record`.
  - `ui` functions: `recording_started(&AppHandle, Region, full) -> Result<(), String>`, `recording_stopped(&AppHandle)`, `set_tray_timer(&AppHandle, Option<u64>)`.
  - `AppState.recording: Mutex<Option<recorder::Recording>>`.

- [ ] **Step 1: Extend the config (test first)**

Add to `store.rs` tests:

```rust
    #[test]
    fn recording_defaults_and_old_configs_load() {
        let c = Config::default();
        assert_eq!(c.fps, 30);
        assert_eq!(c.mic, None);
        assert_eq!(c.shortcuts.record, "Ctrl+Alt+Shift+R");
        let d = tmp("oldcfg");
        let p = d.join("config.toml");
        fs::write(&p, "[shortcuts]\narea = \"Print\"\nscreen = \"Shift+Print\"\nwindow = \"Alt+Print\"\n").unwrap();
        assert_eq!(load_config_from(&p).shortcuts.record, "Ctrl+Alt+Shift+R");
        assert!(recordings_dir(&c).ends_with("Screencasts"));
    }
```

Run `cargo test store` and confirm it fails to compile (the fields don't exist yet). Then:
- Add `#[serde(default)]` to `Shortcuts`, and a `pub record: String` field defaulting to `"Ctrl+Alt+Shift+R"`.
- Add `pub recordings_dir: Option<PathBuf>`, `pub mic: Option<String>` and `pub fps: u8` to `Config`, with defaults `None`, `None`, `30`.
- Add:

```rust
pub fn recordings_dir(c: &Config) -> PathBuf {
    c.recordings_dir.clone().unwrap_or_else(|| {
        dirs::video_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Videos")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Screencasts")
    })
}

pub fn new_recording_path(c: &Config) -> io::Result<PathBuf> {
    let dir = recordings_dir(c);
    fs::create_dir_all(&dir)?;
    Ok(unique_path(&dir, &capture_stem("Recording", Local::now().naive_local()), "mp4"))
}
```

Fix every `Shortcuts { … }` / `Config { … }` literal the compiler flags (e.g. in `settings.rs` tests or defaults).

Run: `cargo test store`
Expected: all store tests pass.

- [ ] **Step 2: `rshot record` (test first)**

In `cli.rs`:
- Add `Record` to `Cmd`.
- Add `assert_eq!(p(&["record"]), Ok(Cmd::Record));` to `parses_every_command`.
- Add the arm `["record"] => Ok(Cmd::Record),`.
- Change `USAGE` to `"usage: rshot [capture area|screen|window] [record] [restore-shortcuts]"`.

Run: `cargo test cli` → passes.

- [ ] **Step 3: Process control in `recorder.rs`**

Append:

```rust
use crate::{err, pipeline, store, ui, AppState};
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State};

static GENERATION: AtomicU64 = AtomicU64::new(1);

pub struct Recording {
    id: u64,
    child: Child,
    mkv: PathBuf,
    mp4: PathBuf,
    started: Instant,
    mic: bool,
}

pub fn is_recording(app: &AppHandle) -> bool {
    app.state::<AppState>().recording.lock().unwrap().is_some()
}

/// ffmpeg's own log, so a failed recording can be diagnosed.
fn ffmpeg_log() -> Stdio {
    let path = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("rshot").join("ffmpeg.log");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    std::fs::File::create(path).map(Stdio::from).unwrap_or_else(|_| Stdio::null())
}

pub fn start(app: &AppHandle, region: Region, full: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.recording.lock().unwrap().is_some() {
        return Err("Already recording".into());
    }
    let ffmpeg = ffmpeg_path().ok_or("ffmpeg isn't available, so recording is disabled")?;
    let cfg = state.config.lock().unwrap().clone();
    let mp4 = store::new_recording_path(&cfg).map_err(err)?;
    let stem = mp4.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mkv = mp4.with_file_name(format!(".{stem}.mkv"));
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
    let args = record_args(region, &display, cfg.fps, cfg.mic.as_deref(), &mkv)?;
    let child = Command::new(&ffmpeg)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(ffmpeg_log())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    let id = GENERATION.fetch_add(1, Ordering::Relaxed);
    *state.recording.lock().unwrap() = Some(Recording { id, child, mkv, mp4, started: Instant::now(), mic: cfg.mic.is_some() });
    ui::recording_started(app, region, full)?;
    watch(app.clone(), id);
    Ok(())
}

/// Ticks the tray timer; if ffmpeg dies on its own, keeps the MKV and tells the user.
fn watch(app: AppHandle, id: u64) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        let state = app.state::<AppState>();
        let mut guard = state.recording.lock().unwrap();
        let Some(rec) = guard.as_mut().filter(|r| r.id == id) else { return };
        if matches!(rec.child.try_wait(), Ok(Some(_))) {
            let rec = guard.take().expect("checked above");
            drop(guard);
            ui::recording_stopped(&app);
            pipeline::notify(&app, &format!("Recording stopped unexpectedly. The raw file is kept at {}", rec.mkv.display()));
            return;
        }
        let secs = rec.started.elapsed().as_secs();
        drop(guard);
        ui::set_tray_timer(&app, Some(secs));
    });
}

fn take(app: &AppHandle) -> Option<Recording> {
    let rec = app.state::<AppState>().recording.lock().unwrap().take();
    if rec.is_some() {
        ui::recording_stopped(app);
    }
    rec
}

/// Asks ffmpeg to finish (`q`), waits up to 10 s, then kills it.
fn end(rec: &mut Recording) {
    if let Some(mut stdin) = rec.child.stdin.take() {
        let _ = stdin.write_all(b"q\n");
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while matches!(rec.child.try_wait(), Ok(None)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = rec.child.kill();
    let _ = rec.child.wait();
}

pub fn stop(app: &AppHandle) -> Result<(), String> {
    let mut rec = take(app).ok_or("Not recording")?;
    end(&mut rec);
    let ffmpeg = ffmpeg_path().ok_or("ffmpeg isn't available")?;
    let ok = Command::new(ffmpeg).args(remux_args(&rec.mkv, &rec.mp4)).stderr(ffmpeg_log()).status().map_err(err)?.success();
    if !ok {
        return Err(format!("Couldn't finish the recording. The raw file is kept at {}", rec.mkv.display()));
    }
    let _ = std::fs::remove_file(&rec.mkv);
    pipeline::finish_video(app, &rec.mp4)
}

pub fn discard(app: &AppHandle) {
    if let Some(mut rec) = take(app) {
        end(&mut rec);
        let _ = std::fs::remove_file(&rec.mkv);
    }
}

#[derive(Serialize)]
pub struct RecordingInfo {
    elapsed_ms: u64,
    mic: bool,
}

#[tauri::command]
pub fn recording_info(state: State<'_, AppState>) -> Option<RecordingInfo> {
    state.recording.lock().unwrap().as_ref().map(|r| RecordingInfo { elapsed_ms: r.started.elapsed().as_millis() as u64, mic: r.mic })
}

#[tauri::command]
pub async fn recording_stop(app: AppHandle) -> Result<(), String> {
    stop(&app).inspect_err(|e| pipeline::notify(&app, e))
}

#[tauri::command]
pub fn recording_discard(app: AppHandle) {
    discard(&app);
}

#[tauri::command]
pub async fn list_mics() -> Vec<Mic> {
    let Some(ffmpeg) = ffmpeg_path() else { return Vec::new() };
    Command::new(ffmpeg)
        .args(["-hide_banner", "-sources", "pulse"])
        .output()
        .map(|o| parse_pulse_sources(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}
```

Add the real-recording integration test to the `tests` module (ignored by default, since it needs a display and the sidecar):

```rust
    /// `cargo test recorder -- --ignored` on a desktop session: records 1 s of the top-left 320×240 and remuxes it.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn records_and_remuxes_one_second() {
        let ffmpeg = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries/rshot-ffmpeg-x86_64-unknown-linux-gnu");
        let dir = std::env::temp_dir().join(format!("rshot-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mkv, mp4) = (dir.join(".R.mkv"), dir.join("R.mp4"));
        let display = std::env::var("DISPLAY").unwrap();
        let mut args = record_args(Region { x: 0, y: 0, w: 320, h: 240 }, &display, 30, None, &mkv).unwrap();
        args.splice(args.len() - 2..args.len() - 2, strs(&["-t", "1"]));
        assert!(std::process::Command::new(&ffmpeg).args(&args).status().unwrap().success());
        assert!(std::process::Command::new(&ffmpeg).args(remux_args(&mkv, &mp4)).status().unwrap().success());
        assert!(std::fs::metadata(&mp4).unwrap().len() > 1000);
    }
```

- [ ] **Step 4: `finish_video`, `Thumb.kind`, and the tray**

In `thumbnail.rs`, add `pub kind: String` to `Thumb`. In `pipeline::finish_capture`, set `kind: "image".into()`. Append to `pipeline.rs`:

```rust
/// After a recording: clipboard (path + file, never an image), last capture, video thumbnail.
pub fn finish_video(app: &AppHandle, path: &std::path::Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();
    let copied = state.clipboard.copy_capture(path, None, cfg.clipboard_mode);
    *state.last_capture.lock().unwrap() = Some(path.to_path_buf());
    *state.thumb.lock().unwrap() = Some(crate::thumbnail::Thumb {
        path: path.display().to_string(),
        display: crate::thumbnail::tildify(path),
        copied: copied.is_ok(),
        kind: "video".into(),
    });
    if cfg.show_thumbnail {
        if let Err(e) = crate::ui::show_thumbnail(app) {
            eprintln!("rshot: thumbnail: {e}");
        }
    } else if copied.is_err() {
        notify(app, "Recording saved, but copying to the clipboard failed");
    }
    Ok(())
}
```

In `ui.rs`:
- Keep the tray's record item so its label can change. In `create_tray`, build `let record = MenuItem::with_id(app, "record", "Record Screen…", true, None::<&str>)?;`, put it after "Capture Window" in the menu, build the tray with `TrayIconBuilder::with_id("main")`, and after `.build(app)?` call `app.manage(TrayRecord(record));`.
- Declare `pub struct TrayRecord(pub MenuItem<tauri::Wry>);`.
- Handle the new menu id: `"record" => Cmd::Record,` (it goes through the same 300 ms-delayed `dispatch`).
- Add:

```rust
pub fn set_tray_timer(app: &AppHandle, secs: Option<u64>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(secs.map(|s| format!("● {}:{:02}", s / 60, s % 60)));
    }
}

/// Task 3 adds the frame and pill windows; the tray part is here.
pub fn recording_started(app: &AppHandle, _region: crate::recorder::Region, _full: bool) -> Result<(), String> {
    if let Some(item) = app.try_state::<TrayRecord>() {
        let _ = item.0.set_text("Stop Recording");
    }
    set_tray_timer(app, Some(0));
    Ok(())
}

pub fn recording_stopped(app: &AppHandle) {
    if let Some(item) = app.try_state::<TrayRecord>() {
        let _ = item.0.set_text("Record Screen…");
    }
    set_tray_timer(app, None);
    close_prefix(app, "recframe");
    close_prefix(app, "pill");
}
```

In `main.rs`:
- Add `pub recording: std::sync::Mutex<Option<recorder::Recording>>,` to `AppState` (with `new()` initialising it to `None`).
- Add `recorder::recording_info, recorder::recording_stop, recorder::recording_discard, recorder::list_mics,` to `generate_handler!`.
- Add a `dispatch` arm. Task 3 routes the not-recording case through the overlay; until then it records the monitor under the pointer:

```rust
        Record => {
            if recorder::is_recording(app) {
                recorder::stop(app)
            } else {
                // Task 3 replaces this with overlay::start(app, "recarea").
                let pos = app.cursor_position().map_err(err);
                pos.and_then(|p| {
                    let frames = capture::grab_all(false)?;
                    let f = &frames[capture::frame_at(&frames, p.x as i32, p.y as i32)];
                    recorder::start(app, recorder::Region { x: f.x, y: f.y, w: f.image.width(), h: f.image.height() }, true)
                })
            }
        }
```

In `src/shared/ipc.ts`:
- Change `Thumb` to `{ path: string; display: string; copied: boolean; kind: 'image' | 'video' }`.
- Add `export type Mic = { id: string; label: string };` and `export const listMics = () => invoke<Mic[]>('list_mics');`.

- [ ] **Step 5: Verify**

1. Run `cargo test && cargo test recorder -- --ignored` (from `src-tauri`). All pass, including `records_and_remuxes_one_second`.
2. `cargo clippy --all-targets -- -D warnings` is clean.
3. With `npm run tauri dev` running:
   - `./src-tauri/target/debug/rshot record`: the tray label shows `● 0:01`, `● 0:02`… and the tray item reads "Stop Recording".
   - Wait about 4 s, then run `./src-tauri/target/debug/rshot record` again: the timer disappears, and `ls -t ~/Videos/Screencasts | head -2` shows `Recording_….mp4` and no `.mkv`.
   - `ffprobe -v error -show_entries format=duration -of csv=p=0 <file>` gives about 4 s.
   - `xclip -selection clipboard -o` prints the MP4 path, and TARGETS lists `text/uri-list` but no `image/png`.
4. Crash path: start a recording, `pkill -f rshot-ffmpeg`, and within 1 s a notification names the kept `.mkv`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri src/shared/ipc.ts && git commit -m "feat(record): recording engine with crash-safe MKV, remux, tray timer and rshot record

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Recording UI — overlay record modes, countdown, frame and pill

**Files:**
- Create: `src/recframe/index.html`, `src/pill/index.html`, `src/pill/main.ts`
- Modify: `src/overlay/index.html`, `src/overlay/main.ts`, `src/shared/ipc.ts`, `src/shared/icons.ts`, `vite.config.ts`, `src-tauri/src/overlay.rs`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `recorder::{start, Region, is_recording, stop}`, `ui::{popup, show_countdown, monitor_at}`, `overlay::{Session, Rect}`, the countdown commands.
- Produces:
  - Command `overlay_record(token, rect: Option<Rect>)`.
  - `AppState.pending_rec: Mutex<Option<(Region, bool)>>`, and `countdown_info` returns 3 for recordings.
  - The overlay modes `'recscreen'` and `'recarea'`.
  - TS: `ipc.overlayRecord`, `ipc.recordingInfo`, `ipc.recordingStop`, `ipc.recordingDiscard`.

- [ ] **Step 1: Rust — record target, countdown hand-off, windows**

In `overlay.rs`, add:

```rust
/// Record the area (or the whole monitor when `rect` is None) after a 3 s countdown.
#[tauri::command]
pub async fn overlay_record(app: AppHandle, window: WebviewWindow, token: u64, rect: Option<Rect>) -> Result<(), String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    let session = {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock().unwrap();
        match guard.take() {
            Some(s) if s.token == token => s,
            other => {
                *guard = other;
                return Ok(());
            }
        }
    };
    ui::hide_overlays(&app);
    let frame = session.frames.into_iter().nth(index).ok_or("that monitor is gone")?;
    let (fw, fh) = frame.image.dimensions();
    let (region, full) = match rect {
        Some(r) => {
            let [x, y, w, h] = capture::clamp_rect(r.x, r.y, r.w, r.h, fw, fh).ok_or("empty selection")?;
            (crate::recorder::Region { x: frame.x + x as i32, y: frame.y + y as i32, w, h }, false)
        }
        None => (crate::recorder::Region { x: frame.x, y: frame.y, w: fw, h: fh }, true),
    };
    *app.state::<AppState>().pending_rec.lock().unwrap() = Some((region, full));
    ui::show_countdown(&app, (region.x + region.w as i32 / 2, region.y + region.h as i32 / 2))
}
```

- Change `countdown_info` to return `3` when `pending_rec` is set: `if state.pending_rec.lock().unwrap().is_some() { return 3; }` as its first line.
- In `countdown_cancel`, also clear `pending_rec`.
- In `countdown_done`, before the existing capture logic, add:

```rust
    let rec = app.state::<AppState>().pending_rec.lock().unwrap().take();
    if let Some((region, full)) = rec {
        ui::close_prefix(&app, "countdown");
        std::thread::sleep(std::time::Duration::from_millis(200));
        let result = crate::recorder::start(&app, region, full);
        if let Err(e) = &result {
            pipeline::notify(&app, e);
        }
        return result;
    }
```

In `ui.rs`, replace `recording_started` with the version that also opens the frame and the pill. Neither opens for full-screen recordings, because they'd be filmed:

```rust
const FRAME_MARGIN: i32 = 8;

pub fn recording_started(app: &AppHandle, r: crate::recorder::Region, full: bool) -> Result<(), String> {
    if let Some(item) = app.try_state::<TrayRecord>() {
        let _ = item.0.set_text("Stop Recording");
    }
    set_tray_timer(app, Some(0));
    if full {
        return Ok(());
    }
    // Dashed frame just outside the area; transparent and click-through.
    let frame = popup(app, "recframe", "recframe/index.html", 100.0, 100.0, false)?;
    frame.set_position(PhysicalPosition::new(r.x - FRAME_MARGIN, r.y - FRAME_MARGIN)).map_err(err)?;
    frame
        .set_size(PhysicalSize::new(r.w + 2 * FRAME_MARGIN as u32, r.h + 2 * FRAME_MARGIN as u32))
        .map_err(err)?;
    frame.set_ignore_cursor_events(true).map_err(err)?;
    frame.show().map_err(err)?;
    // Pill above the area's top-right corner, or below it when there's no room above.
    let m = monitor_at(app, f64::from(r.x), f64::from(r.y))?;
    let s = m.scale_factor();
    let (pw, ph) = (300.0, 52.0);
    let pill = popup(app, "pill", "pill/index.html", pw, ph, false)?;
    let x = r.x + r.w as i32 - (pw * s) as i32;
    let above = r.y - FRAME_MARGIN - (ph * s) as i32 - 4;
    let y = if above >= m.position().y { above } else { r.y + r.h as i32 + FRAME_MARGIN + 4 };
    pill.set_position(PhysicalPosition::new(x.max(m.position().x), y)).map_err(err)?;
    pill.show().map_err(err)
}
```

In `main.rs`:
- Add `pub pending_rec: std::sync::Mutex<Option<(recorder::Region, bool)>>,` to `AppState` and `new()`.
- Register `overlay::overlay_record`.
- Change the `Record` arm of `dispatch` to `if recorder::is_recording(app) { recorder::stop(app) } else { overlay::start(app, "recarea") }`.

- [ ] **Step 2: Frame and pill pages**

`src/recframe/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot recording frame</title>
    <style>
      html, body { margin: 0; height: 100%; background: transparent; overflow: hidden; }
      .f { position: absolute; inset: 3px; border: 2px dashed rgba(255, 69, 58, 0.95); border-radius: 2px; }
      .c { position: absolute; width: 16px; height: 16px; border: 0 solid #ff453a; }
      .c1 { left: 0; top: 0; border-width: 3px 0 0 3px; } .c2 { right: 0; top: 0; border-width: 3px 3px 0 0; }
      .c3 { left: 0; bottom: 0; border-width: 0 0 3px 3px; } .c4 { right: 0; bottom: 0; border-width: 0 3px 3px 0; }
    </style>
  </head>
  <body><div class="f"></div><i class="c c1"></i><i class="c c2"></i><i class="c c3"></i><i class="c c4"></i></body>
</html>
```

(The frame window is 8 px larger than the area on every side, so the 3–5 px dashed line and brackets fall outside the recorded pixels.)

Add to `SPRITE` in `src/shared/icons.ts`:

```html
<symbol id="i-mic" viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></symbol>
<symbol id="i-screen" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/></symbol>
<symbol id="i-recscreen" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/><circle cx="12" cy="10" r="2.6" fill="currentColor" stroke="none"/></symbol>
<symbol id="i-recarea" viewBox="0 0 24 24"><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3"/><circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/></symbol>
```

(Skip `i-screen` if the sprite already has it.)

`src/pill/index.html` (mockup 4a):

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot recording</title>
    <style>
      html, body { height: 100%; overflow: hidden; }
      body { display: flex; align-items: center; justify-content: flex-end; padding: 0 4px; }
      .pill { display: flex; align-items: center; gap: 10px; padding: 5px 5px 5px 12px; border-radius: 99px; font: 600 12px Inter; white-space: nowrap; }
      .dot { width: 9px; height: 9px; border-radius: 50%; background: #ff453a; box-shadow: 0 0 0 4px rgba(255, 69, 58, 0.25); animation: pulse 1.6s ease-in-out infinite; }
      @keyframes pulse { 50% { opacity: 0.45; } }
      #time { font-variant-numeric: tabular-nums; letter-spacing: 0.02em; }
      #mic { color: var(--ok); display: grid; }
      .ghost { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.1); color: #ddd; }
      .ghost .ic { width: 13px; height: 13px; }
      .stop { height: 28px; padding: 0 12px 0 10px; border-radius: 99px; background: #ff453a; display: flex; align-items: center; gap: 7px; color: #fff; font: 600 12px Inter; }
      .stop i { width: 10px; height: 10px; border-radius: 2px; background: #fff; display: block; }
    </style>
  </head>
  <body>
    <div class="pill glass">
      <span class="dot"></span><span id="time">00:00</span>
      <span id="mic" hidden><svg class="ic sm" aria-label="Microphone on"><use href="#i-mic" /></svg></span>
      <button class="ghost" id="discard" data-icon="i-trash" title="Discard recording" aria-label="Discard recording"></button>
      <button class="stop" id="stop"><i></i>Stop</button>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/pill/main.ts`:

```ts
import '../shared/glass.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const info = await ipc.recordingInfo();
const t0 = performance.now() - (info?.elapsed_ms ?? 0);
document.querySelector<HTMLElement>('#mic')!.hidden = !info?.mic;
const time = document.querySelector('#time')!;
const tick = () => {
  const s = Math.floor((performance.now() - t0) / 1000);
  time.textContent = `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
};
tick();
setInterval(tick, 250);
document.querySelector('#stop')!.addEventListener('click', () => void ipc.recordingStop());
document.querySelector('#discard')!.addEventListener('click', () => void ipc.recordingDiscard());
```

Append to `src/shared/ipc.ts`:

```ts
export const overlayRecord = (token: number, rect: Rect | null) => invoke<void>('overlay_record', { token, rect });
export const recordingInfo = () => invoke<{ elapsed_ms: number; mic: boolean } | null>('recording_info');
export const recordingStop = () => invoke<void>('recording_stop');
export const recordingDiscard = () => invoke<void>('recording_discard');
```

In `vite.config.ts`, add `'recframe'` and `'pill'` to `pages`.

- [ ] **Step 3: Overlay record modes**

In `src/overlay/index.html`, insert after the Area button and its following `sep`:

```html
<button class="btn" data-mode="recscreen" data-icon="i-recscreen" title="Record screen" aria-label="Record screen"></button>
<button class="btn" data-mode="recarea" data-icon="i-recarea" title="Record area" aria-label="Record area"></button>
<span class="sep"></span>
```

In `src/shared/ipc.ts`, widen `OverlayInfo['mode']` to `'area' | 'window' | 'screen' | 'recarea' | 'recscreen'`.

In `src/overlay/main.ts`:
- `recarea` behaves exactly like `area` for selection, and `recscreen` like `screen` for highlighting. Wherever the code tests `mode === 'area'`, use `isArea()`; wherever it tests `mode === 'screen'`, use `isScreen()`:

```ts
const isArea = () => mode === 'area' || mode === 'recarea';
const isScreen = () => mode === 'screen' || mode === 'recscreen';
const isRecord = () => mode === 'recarea' || mode === 'recscreen';
```

  (This includes `document.body.dataset.mode`: set it to `isArea() ? 'area' : isScreen() ? 'screen' : 'window'` so the CSS cursors keep working.)
- In `render()`, add: `bar.querySelector<HTMLElement>('[data-act="capture"]')!.textContent = isRecord() ? 'Record' : 'Capture';`.
- Replace `captureNow()` with:

```ts
function captureNow() {
  if (!info) return;
  if (isRecord()) {
    if (mode === 'recscreen') void record(null);
    else if (sel && sel.w >= 2 && sel.h >= 2) void record(sel);
    return;
  }
  if (isScreen()) void capture({ kind: 'screen' });
  else if (mode === 'window') {
    const t = highlight();
    if (t?.id !== undefined) void capture({ kind: 'window', id: t.id, rect: t.rect });
  } else if (sel && sel.w >= 1 && sel.h >= 1) void capture({ kind: 'area', rect: sel });
}

async function record(rect: ipc.Rect | null) {
  if (!info || busy) return;
  busy = true;
  await ipc.overlayRecord(info.token, rect).catch(() => {});
}
```

- In `highlight()`, change `if (mode === 'screen')` to `if (isScreen())`.
- In `mousedown`, change `if (mode !== 'area') return captureNow();` to `if (!isArea()) return captureNow();`.
- `Space` toggles between `area` and `window` only when not recording: `if (!isRecord()) setMode(mode === 'window' ? 'area' : 'window');`.

- [ ] **Step 4: Verify**

Run `npm run build && npm test && (cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test)`: all clean.

With dev running:
1. `./src-tauri/target/debug/rshot record` opens the overlay with **Record area** selected and the button reading "Record".
2. Drag an area and press `⏎`:
   - The overlay hides, and the 3 s ring counts down at the area's centre.
   - Then the dashed red frame appears just outside the area, and the pill sits above its top-right (`00:01`…).
   - Clicks inside the frame reach the app underneath.
   - The tray shows `● 0:0x`.
3. Click **Stop**:
   - The frame and pill disappear, and the video thumbnail appears (Task 4 styles it; for now it may show a blank image).
   - The saved MP4's size equals the area's even-rounded size (`ffprobe -v error -show_entries stream=width,height -of csv=p=0 <file>`).
   - Scrub through the file (`ffplay -autoexit <file>` or `eog`): the dashed frame never appears in the video.
4. Record screen: after the countdown there is no frame or pill, only the tray timer. Running `rshot record` again stops and saves.
5. **Discard** (trash on the pill) removes the MKV, and no MP4 is created.
6. `Esc` during the countdown cancels, and nothing records.

- [ ] **Step 5: Commit**

```bash
git add src src-tauri vite.config.ts && git commit -m "feat(record): overlay record modes, countdown, dashed frame and control pill

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Video thumbnail + video trim editor

**Files:**
- Create: `src/video/index.html`, `src/video/video.css`, `src/video/main.ts`, `src/video/trim.ts`, `src/video/trim.test.ts`
- Modify: `src-tauri/Cargo.toml` (`protocol-asset`), `src-tauri/tauri.conf.json` (asset scope), `src-tauri/src/main.rs`, `src-tauri/src/ui.rs` (`open_editor` picks the page by extension), `src-tauri/src/editor.rs` (`trim_video`), `src/thumbnail/{index.html,thumbnail.css,main.ts}`, `src/shared/ipc.ts`, `vite.config.ts`

**Interfaces:**
- Consumes: Plan 2's `ui::open_editor`/`AppState.editors`/`editor_info`/`editor_delete`, `recorder::{ffmpeg_path, trim_args}`, `Clipboard::copy_capture`.
- Produces:
  - Command `trim_video(start: f64, end: f64, mute: bool)`.
  - `trim.ts` exports `fmt(s: number): string`, `clampTrim(start, end, dur, minLen): [number, number]` and `timeAt(px, width, dur): number`.
  - The asset protocol, enabled for the capture folders.

- [ ] **Step 1: Pure trim helpers (test first)**

`src/video/trim.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { clampTrim, fmt, timeAt } from './trim';

describe('trim helpers', () => {
  it('formats m:ss.t', () => {
    expect(fmt(12.94)).toBe('0:12.9');
    expect(fmt(75)).toBe('1:15.0');
  });
  it('keeps the range inside the video and at least minLen long', () => {
    expect(clampTrim(-1, 50, 34, 0.5)).toEqual([0, 34]);
    expect(clampTrim(10, 10.2, 34, 0.5)).toEqual([10, 10.5]);
    expect(clampTrim(33.9, 34, 34, 0.5)).toEqual([33.5, 34]);
  });
  it('maps pixels to seconds', () => {
    expect(timeAt(50, 200, 34)).toBe(8.5);
    expect(timeAt(-5, 200, 34)).toBe(0);
    expect(timeAt(500, 200, 34)).toBe(34);
  });
});
```

`src/video/trim.ts`:

```ts
/** m:ss.t */
export function fmt(s: number): string {
  const m = Math.floor(s / 60);
  return `${m}:${(s - m * 60).toFixed(1).padStart(4, '0')}`;
}

/** Keeps [start, end] inside [0, dur] and at least `minLen` long. */
export function clampTrim(start: number, end: number, dur: number, minLen: number): [number, number] {
  let s = Math.max(0, Math.min(start, dur));
  let e = Math.max(0, Math.min(end, dur));
  if (e - s < minLen) {
    if (s + minLen <= dur) e = s + minLen;
    else s = Math.max(0, e - minLen);
  }
  return [s, e];
}

export function timeAt(px: number, width: number, dur: number): number {
  return Math.max(0, Math.min(dur, (px / width) * dur));
}
```

Run: `npm test -- src/video`. The tests pass; the helpers are simple enough that writing them first is enough. Keep the test file.

- [ ] **Step 2: Asset protocol, page routing, `trim_video`**

- `cargo add tauri@2.11 --features tray-icon,macos-private-api,protocol-asset` (keeps the existing features and adds `protocol-asset`).
- In `tauri.conf.json`, set `app.security` to:

  ```json
  "security": { "csp": null, "assetProtocol": { "enable": true, "scope": ["$PICTURE/**", "$VIDEO/**"] } }
  ```

- In `main.rs` `setup`, allow custom folders too:

  ```rust
  {
      let c = app.state::<AppState>().config.lock().unwrap().clone();
      let scope = app.asset_protocol_scope();
      let _ = scope.allow_directory(store::screenshots_dir(&c), true);
      let _ = scope.allow_directory(store::recordings_dir(&c), true);
  }
  ```

  Then make the same two `allow_directory` calls at the end of `settings::set_settings`, after saving.
- In `ui::open_editor`, pick the page by extension: `let page = if path.extension().is_some_and(|e| e == "mp4") { "video/index.html" } else { "editor/index.html" };`, and use `page` in `WebviewUrl::App(page.into())`.
- Make the thumbnail's path guard accept the recordings folder too. In `thumbnail::guard`, compute `let vdir = store::recordings_dir(&cfg).canonicalize().ok();` and accept `p` if it `starts_with` either folder. (Read the config once into `cfg`.)
- In `editor.rs`, add:

```rust
/// Re-encodes [start, end] to a temp file, then renames it over the original (path unchanged) and re-copies.
#[tauri::command]
pub async fn trim_video(app: tauri::AppHandle, window: WebviewWindow, start: f64, end: f64, mute: bool) -> Result<(), String> {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let p = editor_path(&state, &window)?;
    let ffmpeg = crate::recorder::ffmpeg_path().ok_or("ffmpeg isn't available")?;
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = p.with_file_name(format!(".{stem}.trim.mp4"));
    let ok = std::process::Command::new(ffmpeg)
        .args(crate::recorder::trim_args(&p, start, end, mute, &tmp))
        .status()
        .map_err(err)?
        .success();
    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return Err("ffmpeg couldn't trim the video".into());
    }
    std::fs::rename(&tmp, &p).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&p, None, mode)
}
```

- Register `editor::trim_video` in `generate_handler!`.
- Append to `ipc.ts`: `export const trimVideo = (start: number, end: number, mute: boolean) => invoke<void>('trim_video', { start, end, mute });`.
- Add `'video'` to the Vite `pages`.

- [ ] **Step 3: Video thumbnail** (mockup 4c)

In `src/thumbnail/index.html`, add inside `.shot` after the `<img>`:

```html
<video id="vid" muted preload="metadata" hidden></video>
<span class="play" hidden><i></i></span><span class="dur" hidden></span>
```

Append to `thumbnail.css`:

```css
.shot video { max-width: 100%; max-height: 100%; }
.play { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 36px; height: 36px; border-radius: 50%; background: rgba(20, 20, 24, 0.6); display: grid; place-items: center; }
.play i { width: 0; height: 0; border-left: 11px solid #fff; border-top: 7px solid transparent; border-bottom: 7px solid transparent; margin-left: 3px; }
.dur { position: absolute; right: 7px; bottom: 7px; font: 600 10px Inter; background: rgba(20, 20, 24, 0.72); color: #fff; border-radius: 5px; padding: 2px 6px; }
```

In `src/thumbnail/main.ts`:
- `import { convertFileSrc } from '@tauri-apps/api/core';`
- Replace the line that sets `img.src` with:

```ts
  const vid = document.querySelector<HTMLVideoElement>('#vid')!;
  if (t.kind === 'video') {
    img.hidden = true;
    vid.hidden = false;
    vid.src = convertFileSrc(t.path);
    vid.addEventListener('loadedmetadata', () => {
      vid.currentTime = Math.min(0.1, vid.duration / 2);
      const d = document.querySelector<HTMLElement>('.dur')!;
      d.hidden = false;
      d.textContent = `${Math.floor(vid.duration / 60)}:${String(Math.round(vid.duration % 60)).padStart(2, '0')}`;
    });
    document.querySelector<HTMLElement>('.play')!.hidden = false;
  } else {
    img.src = URL.createObjectURL(new Blob([await ipc.readCapture(t.path)], { type: 'image/png' }));
  }
```

- Make `dragIcon()` draw from whichever is visible: `const src: CanvasImageSource = t.kind === 'video' ? vid : img;`, with natural sizes from `vid.videoWidth`/`vid.videoHeight` for videos. Move `dragIcon` inside `run()` so it can see `t` and `vid`.

- [ ] **Step 4: Video editor page** (mockup 4d)

`src/video/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot video</title>
  </head>
  <body>
    <div class="ed">
      <header data-tauri-drag-region>
        <span class="title" id="title"></span><span class="meta" id="meta"></span><span class="sp" data-tauri-drag-region></span>
        <button class="ib" id="delete" data-icon="i-trash" title="Delete recording" aria-label="Delete recording"></button>
        <button class="b2" id="copy" data-icon="i-copy" title="Copy path + file">Copy</button>
        <button class="b1" id="done" title="Save and close (Enter)">Done</button>
        <button class="wx" id="close" data-icon="i-x" aria-label="Close"></button>
      </header>
      <div class="stage"><video id="v" preload="auto"></video><button class="bigplay" id="bigplay" aria-label="Play"><i></i></button></div>
      <div class="tl">
        <div class="tlrow">
          <button class="pbtn" id="play" aria-label="Play or pause (Space)"><i></i></button>
          <div class="strip" id="strip">
            <canvas id="frames"></canvas>
            <div class="out" id="out-l"></div><div class="out" id="out-r"></div>
            <div class="trim" id="trim"><span class="grip l" data-grip="start"></span><span class="grip r" data-grip="end"></span></div>
            <div class="head" id="head"></div>
          </div>
        </div>
        <div class="tlmeta">
          <span id="readout"></span>
          <span class="right">
            <label class="tog">Mute audio<button class="switch" role="switch" id="mute" aria-label="Mute audio"></button></label>
            <span class="chip"><span class="ok"><svg class="ic" aria-hidden="true"><use href="#i-check" /></svg></span><span id="path"></span>
              <button class="cp" id="reveal" data-icon="i-folder" title="Show in folder" aria-label="Show in folder"></button></span>
          </span>
        </div>
      </div>
      <div id="modal" hidden><div class="glass box"><p id="modal-text"></p><div class="acts" id="modal-acts"></div></div></div>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/video/video.css`:

```css
html, body { height: 100%; overflow: hidden; }
.ed { position: relative; height: 100%; display: flex; flex-direction: column; border-radius: 12px; overflow: hidden; background: var(--surface); border: 1px solid #34343a; }
header { height: 48px; flex-shrink: 0; background: var(--surface-2); border-bottom: 1px solid var(--line); display: flex; align-items: center; padding: 0 10px 0 16px; gap: 8px; }
.title { font: 600 13px Inter; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.meta { font: 500 11.5px Inter; color: var(--text-3); margin-left: 8px; white-space: nowrap; }
.sp { flex: 1; align-self: stretch; }
.ib { width: 32px; height: 30px; display: grid; place-items: center; border-radius: 8px; color: #d9d9e0; }
.ib:hover { background: rgba(255, 255, 255, 0.1); }
.wx { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.1); margin-left: 8px; }
.wx .ic { width: 12px; height: 12px; }
.stage { flex: 1; min-height: 0; display: grid; place-items: center; background: #141417; position: relative; padding: 24px; }
#v { max-width: 100%; max-height: 100%; border-radius: 3px; box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5); }
.bigplay { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 58px; height: 58px; border-radius: 50%; background: rgba(30, 30, 34, 0.55); backdrop-filter: blur(10px); display: grid; place-items: center; border: 1px solid rgba(255, 255, 255, 0.15); }
.bigplay i, .pbtn i { width: 0; height: 0; border-left: 17px solid #fff; border-top: 11px solid transparent; border-bottom: 11px solid transparent; margin-left: 5px; }
.tl { flex-shrink: 0; background: var(--surface-2); border-top: 1px solid var(--line); padding: 12px 16px; }
.tlrow { display: flex; align-items: center; gap: 12px; }
.pbtn { width: 34px; height: 34px; border-radius: 50%; background: rgba(255, 255, 255, 0.1); display: grid; place-items: center; flex-shrink: 0; }
.pbtn i { border-left-width: 11px; border-top-width: 7px; border-bottom-width: 7px; margin-left: 3px; }
.pbtn.playing i, .bigplay.playing i { width: 10px; height: 12px; border: 0; margin: 0; background: linear-gradient(90deg, #fff 0 35%, transparent 35% 65%, #fff 65%); }
.bigplay.playing { opacity: 0; }
.strip { position: relative; flex: 1; height: 52px; }
#frames { position: absolute; inset: 0; width: 100%; height: 100%; border-radius: 8px; background: #000; }
.out { position: absolute; top: 0; bottom: 0; background: rgba(10, 10, 12, 0.66); }
#out-l { left: 0; border-radius: 8px 0 0 8px; } #out-r { right: 0; border-radius: 0 8px 8px 0; }
.trim { position: absolute; top: -2px; bottom: -2px; border: 3px solid #ffd60a; border-left-width: 12px; border-right-width: 12px; border-radius: 8px; pointer-events: none; }
.grip { position: absolute; top: -3px; bottom: -3px; width: 18px; cursor: ew-resize; pointer-events: auto; }
.grip.l { left: -15px; } .grip.r { right: -15px; }
.head { position: absolute; top: -8px; bottom: -8px; width: 2px; margin-left: -1px; background: #fff; border-radius: 1px; box-shadow: 0 0 4px rgba(0, 0, 0, 0.6); pointer-events: none; }
.head::before { content: ''; position: absolute; top: -4px; left: -5px; width: 12px; height: 12px; border-radius: 50%; background: #fff; }
.tlmeta { display: flex; justify-content: space-between; align-items: center; margin-top: 12px; padding-left: 46px; font: 500 11.5px Inter; color: rgba(255, 255, 255, 0.55); font-variant-numeric: tabular-nums; gap: 12px; }
.tlmeta b { color: #fff; font-weight: 600; }
.right { display: flex; align-items: center; gap: 14px; min-width: 0; }
.tog { display: flex; align-items: center; gap: 8px; color: rgba(255, 255, 255, 0.8); }
.chip { display: flex; align-items: center; gap: 8px; height: 26px; padding: 0 6px 0 8px; border-radius: 7px; background: rgba(255, 255, 255, 0.06); min-width: 0; }
.chip #path { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.chip .ok { width: 15px; height: 15px; border-radius: 50%; background: var(--ok); display: grid; place-items: center; color: #063; flex-shrink: 0; }
.chip .ok .ic { width: 9px; height: 9px; stroke-width: 3.2; }
.cp { width: 22px; height: 22px; border-radius: 6px; display: grid; place-items: center; color: rgba(255, 255, 255, 0.7); }
.cp .ic { width: 14px; height: 14px; }
#modal { position: absolute; inset: 0; z-index: 20; display: grid; place-items: center; background: rgba(0, 0, 0, 0.45); }
#modal .box { width: 360px; padding: 20px; border-radius: 14px; font: 500 13px Inter; line-height: 1.5; }
#modal .acts { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.danger { background: var(--danger) !important; color: #fff; }
.saving { pointer-events: none; opacity: 0.6; }
```

`src/video/main.ts`:

```ts
import '../shared/glass.css';
import './video.css';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import { clampTrim, fmt, timeAt } from './trim';

mountIcons();
const $ = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const v = $<HTMLVideoElement>('#v');
const strip = $('#strip');
const info = await ipc.editorInfo();
$('#title').textContent = info.name;
$('#path').textContent = info.display;
v.src = convertFileSrc(info.path);
await new Promise((ok) => v.addEventListener('loadedmetadata', ok, { once: true }));
const dur = v.duration;
let [start, end] = [0, dur];
let mute = false;
$('#meta').textContent = `${fmt(dur)} · ${v.videoWidth} × ${v.videoHeight} · MP4`;

const changed = () => start > 0.05 || end < dur - 0.05 || mute;

function render() {
  const w = strip.clientWidth;
  const x = (t: number) => (t / dur) * w;
  $('#out-l').style.width = `${x(start)}px`;
  $('#out-r').style.width = `${w - x(end)}px`;
  Object.assign($('#trim').style, { left: `${x(start)}px`, width: `${x(end) - x(start)}px` });
  $('#head').style.left = `${x(v.currentTime)}px`;
  $('#readout').innerHTML = `<b>${fmt(v.currentTime)}</b> / ${fmt(dur)} &nbsp;·&nbsp; keeping <b>${fmt(start)} → ${fmt(end)}</b> (${(end - start).toFixed(1)} s)`;
  $('#mute').classList.toggle('on', mute);
  $('#mute').setAttribute('aria-checked', String(mute));
  $('#play').classList.toggle('playing', !v.paused);
  $('#bigplay').classList.toggle('playing', !v.paused);
}

/** Filmstrip: seek a second, hidden video through 12 evenly spaced frames. */
async function filmstrip() {
  const c = $<HTMLCanvasElement>('#frames');
  c.width = strip.clientWidth * devicePixelRatio;
  c.height = 52 * devicePixelRatio;
  const g = c.getContext('2d')!;
  const probe = document.createElement('video');
  probe.muted = true;
  probe.src = v.src;
  await new Promise((ok) => probe.addEventListener('loadeddata', ok, { once: true }));
  const n = 12;
  const fw = c.width / n;
  for (let i = 0; i < n; i++) {
    probe.currentTime = Math.min(dur - 0.05, ((i + 0.5) / n) * dur);
    await new Promise((ok) => probe.addEventListener('seeked', ok, { once: true }));
    const h = c.height;
    const w = Math.min(fw, (probe.videoWidth / probe.videoHeight) * h);
    g.drawImage(probe, i * fw + (fw - w) / 2, 0, w, h);
  }
}

function toggle() {
  if (v.paused) {
    if (v.currentTime < start || v.currentTime >= end) v.currentTime = start;
    void v.play();
  } else v.pause();
}

v.addEventListener('timeupdate', () => {
  if (!v.paused && v.currentTime >= end) {
    v.pause();
    v.currentTime = start;
  }
  render();
});
v.addEventListener('play', render);
v.addEventListener('pause', render);
$('#play').addEventListener('click', toggle);
$('#bigplay').addEventListener('click', toggle);
v.addEventListener('click', toggle);
$('#mute').addEventListener('click', () => {
  mute = !mute;
  render();
});

// Dragging a grip moves that end; dragging elsewhere on the strip scrubs.
strip.addEventListener('pointerdown', (e) => {
  const grip = (e.target as HTMLElement).dataset.grip as 'start' | 'end' | undefined;
  strip.setPointerCapture(e.pointerId);
  const at = (ev: PointerEvent) => timeAt(ev.clientX - strip.getBoundingClientRect().left, strip.clientWidth, dur);
  const apply = (ev: PointerEvent) => {
    const t = at(ev);
    if (grip === 'start') [start, end] = clampTrim(Math.min(t, end - 0.5), end, dur, 0.5);
    else if (grip === 'end') [start, end] = clampTrim(start, Math.max(t, start + 0.5), dur, 0.5);
    v.currentTime = grip === 'end' ? end : grip === 'start' ? start : t;
    render();
  };
  apply(e);
  const up = () => {
    strip.removeEventListener('pointermove', apply);
    strip.removeEventListener('pointerup', up);
  };
  strip.addEventListener('pointermove', apply);
  strip.addEventListener('pointerup', up);
});

function ask(text: string, buttons: { label: string; value: string; primary?: boolean; danger?: boolean }[]): Promise<string> {
  return new Promise((resolve) => {
    $('#modal-text').textContent = text;
    $('#modal-acts').replaceChildren(
      ...buttons.map((b) => {
        const el = document.createElement('button');
        el.className = b.primary ? 'b1' : 'b2';
        if (b.danger) el.classList.add('danger');
        el.textContent = b.label;
        el.onclick = () => {
          $('#modal').hidden = true;
          resolve(b.value);
        };
        return el;
      }),
    );
    $('#modal').hidden = false;
  });
}

let busy = false;
async function done() {
  if (busy) return;
  busy = true;
  document.body.classList.add('saving');
  try {
    v.pause();
    if (changed()) await ipc.trimVideo(start, end, mute);
    else await ipc.retryCopy(info.path);
    await ipc.closeWindow();
  } catch (e) {
    await ask(`Couldn't save: ${e}`, [{ label: 'OK', value: 'ok', primary: true }]);
  }
  document.body.classList.remove('saving');
  busy = false;
}

async function requestClose() {
  if (!changed()) return ipc.closeWindow();
  const r = await ask(`Save the trimmed ${info.name}?`, [
    { label: 'Discard', value: 'discard', danger: true },
    { label: 'Cancel', value: 'cancel' },
    { label: 'Save', value: 'save', primary: true },
  ]);
  if (r === 'discard') await ipc.closeWindow();
  else if (r === 'save') await done();
}

$('#done').addEventListener('click', () => void done());
$('#copy').addEventListener('click', () => void ipc.retryCopy(info.path));
$('#close').addEventListener('click', () => void requestClose());
$('#reveal').addEventListener('click', () => void ipc.revealCapture(info.path));
$('#delete').addEventListener('click', async () => {
  const r = await ask(`Delete ${info.name}? This can't be undone.`, [
    { label: 'Cancel', value: 'cancel' },
    { label: 'Delete', value: 'delete', primary: true, danger: true },
  ]);
  if (r === 'delete') await ipc.editorDelete();
});
void getCurrentWindow().onCloseRequested(async (e) => {
  e.preventDefault();
  await requestClose();
});
addEventListener('keydown', (e) => {
  if (!$('#modal').hidden) return;
  if (e.key === ' ') {
    e.preventDefault();
    toggle();
  } else if (e.key === 'Enter') void done();
  else if (e.key === 'Escape') void requestClose();
});
addEventListener('resize', () => {
  render();
  void filmstrip();
});

render();
void filmstrip();
```

- [ ] **Step 5: Verify**

Run `npm run build && npm test && (cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test)`: all clean.

With dev running, record about 10 s (area mode), then check each item against mockups 4c and 4d:
- The video thumbnail shows the first frame, a play glyph and a `0:10` badge. "Path copied" shows the `~/Videos/Screencasts/…mp4` path.
- Clicking the thumbnail opens the video editor, with the filmstrip filled with 12 frames.
- Space plays and pauses. Dragging on the strip scrubs, and the playhead follows.
- Drag the yellow handles to keep 2 s–6 s. The readout reads `keeping 0:02.0 → 0:06.0 (4.0 s)`.
- **Done** shows the saving state and closes. `ffprobe` duration is now about 4.0 s (±0.1), and the path is unchanged. The clipboard holds the path (TARGETS include `text/uri-list`, no `image/png`).
- Reopen via tray → Open Last Capture. **Mute audio** + **Done** (on a mic recording) gives a file with no audio stream (`ffprobe -show_streams` lists video only).
- Close with a changed trim → the "Save the trimmed…?" dialog → Cancel keeps the window open.

- [ ] **Step 6: Commit**

```bash
git add src src-tauri vite.config.ts && git commit -m "feat(record): video thumbnail and trim editor over the asset protocol

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Recording settings, mic picker, record-shortcut takeover

**Files:**
- Modify:
  - `src-tauri/src/settings.rs`, `src-tauri/src/overlay.rs` (`OverlayOptions` + `set_overlay_options`)
  - `src-tauri/src/shortcuts/{gnome.rs,mod.rs}`
  - `src/settings/{index.html,main.ts}`, `src/overlay/options.ts`, `src/onboarding/main.ts`, `src/shared/ipc.ts`
- Test: extend `gvariant` tests only if helpers change (none expected); the rest is verified manually

**Interfaces:**
- Consumes: `Config.{recordings_dir, mic, fps, shortcuts.record}`, `ipc.listMics`.
- Produces:
  - `Settings` gains `recordings_dir: String`, `mic: Option<String>`, `fps: u8`, and `shortcuts.record` comes through `Shortcuts`.
  - `OverlayOptions` gains `mic: Option<String>`.
  - GNOME takeover covers `show-screen-recording-ui` and binds `rshot record`.

- [ ] **Step 1: Rust**

- `gnome.rs`:
  - `TAKEN_KEYS` becomes `["show-screenshot-ui", "screenshot", "screenshot-window", "show-screen-recording-ui"]`.
  - `OURS` gains `("record", "record")`.
  - The accelerator match gains `"record" => &cfg.shortcuts.record,`.
  - Backup must now fill in **missing** keys, so users who took over before this plan keep their original record binding:

```rust
    let backup = cfg.gnome_backup.get_or_insert_with(BTreeMap::new);
    for k in TAKEN_KEYS {
        if !backup.contains_key(k) {
            backup.insert(k.to_string(), gs(&["get", SHELL, k])?);
        }
    }
```

  (This replaces the old `if cfg.gnome_backup.is_none() { … }` block.)
- `shortcuts::manual_commands` gains `(c.shortcuts.record.clone(), format!("\"{exe}\" record"))`.
- `settings::Settings` gains `pub recordings_dir: String, pub mic: Option<String>, pub fps: u8`:
  - `snapshot` fills them in (`store::recordings_dir(c).display().to_string()`, `c.mic.clone()`, `c.fps`).
  - `set_settings` applies them (`c.recordings_dir = dir_setting_for(...)`; see below; `c.mic = settings.mic.clone(); c.fps = if settings.fps == 60 { 60 } else { 30 };`).
- `store::dir_setting` only knows the screenshots default. Add `pub fn recordings_dir_setting(chosen: &str) -> Option<PathBuf> { let p = PathBuf::from(chosen); (p != recordings_dir(&Config::default())).then_some(p) }` and use it for `recordings_dir`.
- `overlay::OverlayOptions` gains `pub mic: Option<String>`. `from_config` sets it and `set_overlay_options` stores it.

- [ ] **Step 2: Settings page** (mockup 05c)

In `src/settings/index.html`:
- Add a row to General after "Screenshots folder":

  ```html
  <div class="row"><span class="l">Recordings folder</span><button class="fld" id="rfolder"><svg class="ic" aria-hidden="true"><use href="#i-folder" /></svg><span></span></button></div>
  ```

- Add a row to Shortcuts:

  ```html
  <div class="row"><span class="l">Record screen</span><button class="keys" data-shortcut="record"></button></div>
  ```

- Add a Recording section before the footer:

  ```html
  <section>
    <h5>Recording</h5>
    <div class="box">
      <div class="row"><span class="l">Microphone</span><select class="fld" id="mic" aria-label="Microphone"><option value="">None</option></select></div>
      <div class="row"><span class="l">Frame rate</span><div class="seg" id="fps"><button data-fps="30">30 fps</button><button data-fps="60">60 fps</button></div></div>
    </div>
  </section>
  ```

Append to `settings.css`:

```css
select.fld { appearance: none; border: 0; color: rgba(255, 255, 255, 0.85); padding-right: 28px; background: var(--fill) url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E") no-repeat right 8px center / 12px; max-width: 320px; }
select.fld option { background: #232328; }
```

In `src/shared/ipc.ts`:
- `Shortcuts` gains `record: string`.
- `Settings` gains `recordings_dir: string; mic: string | null; fps: number`.
- `OverlayOptions` gains `mic: string | null`.

In `src/settings/main.ts`:
- `render()` also sets `#rfolder span`, the `#fps` buttons' `.on` (`Number(b.dataset.fps) === s.fps`), and `#mic`'s value (`s.mic ?? ''`).
- The click handler handles `b.id === 'rfolder'` (pick a folder → `s.recordings_dir`) and `b.dataset.fps` (`s.fps = Number(b.dataset.fps)`).
- Add:

```ts
const micSel = document.querySelector<HTMLSelectElement>('#mic')!;
for (const m of await ipc.listMics()) micSel.add(new Option(m.label, m.id));
micSel.addEventListener('change', async () => {
  s.mic = micSel.value || null;
  await save();
});
```

In `src/onboarding/main.ts`, add `['Record screen', s.shortcuts.record]` to `rows`.

- [ ] **Step 3: Overlay popover mic field** (mockup 2c)

In `src/overlay/options.ts`, add a fourth grid cell after the `toggles` div:

```html
<div><div class="lbl">Microphone (recording)</div><select class="field" id="mic" aria-label="Microphone"><option value="">None</option></select></div>
```

Change `.toggles` to span a single column (`grid-column: 1`) in `overlay.css` so the mic sits to its right, as in the mockup. Populate the mic list once:

```ts
const micSel = pop.querySelector<HTMLSelectElement>('#mic')!;
void ipc.listMics().then((mics) => mics.forEach((m) => micSel.add(new Option(m.label, m.id))));
micSel.addEventListener('change', async () => {
  if (!opts) return;
  opts.mic = micSel.value || null;
  await ipc.setOverlayOptions(opts);
});
```

In `renderOptions`, add: `micSel.value = o.mic ?? '';`.

- [ ] **Step 4: Verify**

Run `npm run build && npm test && (cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test)`: clean.

With dev running:
- Settings shows Recordings folder, the Record screen shortcut, and Recording → Microphone (lists this machine's inputs, e.g. "SteelSeries Arctis 7 Chat", "Webcam Vitade AF…") → 60 fps.
- Pick a mic and record 3 s: `ffprobe` shows an `aac` stream and `r_frame_rate=60/1`. Set it back to None and 30 fps.
- The overlay Options popover shows the Microphone field, and it stays in sync with Settings.
- With takeover on:
  - `gsettings get org.gnome.shell.keybindings show-screen-recording-ui` → `@as []`.
  - `python3 scripts/xdo.py key ctrl+alt+shift+r` opens the record overlay.
  - `rshot restore-shortcuts` restores `['<Ctrl><Shift><Alt>R']` along with the other three keys.
  - Finish with shortcuts restored (Environment notes).

- [ ] **Step 5: Commit**

```bash
git add src src-tauri && git commit -m "feat(record): microphone, frame rate and recordings folder settings; record shortcut takeover

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Package with the bundled ffmpeg and update the docs

**Files:**
- Modify: `README.md`, `src-tauri/tauri.conf.json` (only if the check below fails)

**Interfaces:** none. Deliverable: the `.deb` contains `/usr/bin/rshot-ffmpeg`, and recording works from the release build.

- [ ] **Step 1: Build and inspect the package**

```bash
npm run tauri build
D=src-tauri/target/release/bundle/deb/rshot_0.1.0_amd64.deb
dpkg-deb -c "$D" | grep -E 'usr/bin/(rshot|rshot-ffmpeg)$'
dpkg-deb -I "$D" | grep -E 'Depends|Installed-Size'
ls -la "$D"
```

Expected:
- `usr/bin/rshot` and `usr/bin/rshot-ffmpeg` are both listed.
- `Depends` contains only webkit/gtk/appindicator/pipewire/gbm libraries and no `ffmpeg`.
- The size is in the tens of MB.

- [ ] **Step 2: Run the release build as installed**

```bash
pkill -x rshot; mkdir -p /tmp/rshot-pkg && dpkg-deb -x "$D" /tmp/rshot-pkg
PATH=/usr/sbin:/usr/bin:/sbin:/bin /tmp/rshot-pkg/usr/bin/rshot > /tmp/rshot-pkg.log 2>&1 &
sleep 3; /tmp/rshot-pkg/usr/bin/rshot record
```

Then select an area, Record, wait 3 s and Stop. Expected: an MP4 is saved, even though no system `ffmpeg` is involved. `grep -c rshot-ffmpeg /proc/$(pgrep -f 'rshot-ffmpeg' | head -1)/cmdline` shows the bundled binary was used during the recording.

Finally, stop the app: `pkill -f /tmp/rshot-pkg/usr/bin/rshot`.

- [ ] **Step 3: README**

Add a "Record the screen" section after the shortcuts table:

```markdown
## Record the screen

`Ctrl+Alt+Shift+R` (or the record buttons in the `Print` toolbar) → pick an area or the whole screen → 3-second
countdown → recording. Stop with the pill's **Stop**, the tray's **Stop Recording**, or the same shortcut again.
Recordings are saved as `~/Videos/Screencasts/Recording_YYYY-MM-DD_HH-MM-SS.mp4` (H.264, optional microphone),
their path is copied, and clicking the thumbnail opens a trimmer. rshot ships its own ffmpeg (`rshot-ffmpeg`; see
`THIRD_PARTY.md`), so nothing else needs installing.
```

In the Develop section, add `scripts/fetch-ffmpeg.sh   # once: downloads the bundled ffmpeg sidecar` before `npm run tauri dev`.

- [ ] **Step 4: Commit**

```bash
git add README.md src-tauri/tauri.conf.json && git commit -m "docs: recording usage; verify the .deb bundles rshot-ffmpeg

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Self-review notes

- **Spec coverage:**

  | Spec item | Covered by |
  |---|---|
  | §2.1 record row (Linux) | Tasks 2, 3, 5 |
  | §2.6 flow, frame, pill, full-screen tray collapse, stop methods | Tasks 2–3 |
  | §2.6 MKV → MP4 | Task 2 |
  | §2.6 mic / fps settings | Task 5 |
  | §2.7 trim editor | Task 4 |
  | §2.8 Recording settings group | Task 5 |
  | §3.1 bundled ffmpeg | Tasks 1, 6 |
  | §4 "ffmpeg missing" (commands return a clear error that's notified) | Task 2 |
  | §4 "ffmpeg exits mid-recording" (watchdog) | Task 2 |

- **Not built:** the mic level meter (the pill shows a mic icon), and pause/resume. Both are recorded as rulings.
