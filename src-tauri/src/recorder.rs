//! Screen recording through the bundled static ffmpeg (`rshot-ffmpeg`): ffmpeg writes a hidden
//! MKV (it survives a crash), which Stop remuxes into the MP4.

use crate::{err, pipeline, store, ui, AppState};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State};

/// A recorded area. `x`/`y`/`w`/`h` are physical desktop pixels (macOS: the monitor's origin in
/// points × its scale, plus image pixels). `screen` is its monitor's index in `Monitor::all()`
/// order, `lx`/`ly` its top-left in that monitor's image pixels, and `fw`/`fh` the size of that
/// image. avfoundation crops in proportion to `fw`/`fh`, so the cut is right whether its
/// "Capture screen N" frames come in pixels or in points.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub screen: usize,
    pub lx: u32,
    pub ly: u32,
    pub fw: u32,
    pub fh: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Mic {
    pub id: String,
    pub label: String,
}

/// Every ffmpeg rshot runs starts here. Windows: without a console window (a release rshot has
/// none, so each ffmpeg would open one, and a recording's would be filmed); piped stdin still works.
pub fn ffmpeg_command(ffmpeg: &Path) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(ffmpeg);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(windows))]
    Command::new(ffmpeg)
}

/// The sidecar next to our executable (installed or dev), else `ffmpeg` on PATH.
pub fn ffmpeg_path() -> Option<PathBuf> {
    let (name, exe) = if cfg!(windows) {
        ("rshot-ffmpeg.exe", "ffmpeg.exe")
    } else {
        ("rshot-ffmpeg", "ffmpeg")
    };
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        if runnable(&dir.join(name)) {
            return Some(dir.join(name));
        }
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.join(exe))
        .find(|p| runnable(p))
}

fn runnable(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    p.is_file()
}

#[cfg(target_os = "linux")]
pub const NO_FFMPEG: &str =
    "Recording needs ffmpeg, which wasn't found. Reinstall rshot, or install ffmpeg (sudo apt install ffmpeg).";
// Not Homebrew's: an app started from Finder or at login doesn't have it on its PATH.
#[cfg(target_os = "macos")]
pub const NO_FFMPEG: &str = "Recording needs ffmpeg, which wasn't found. Reinstall rshot.";
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub const NO_FFMPEG: &str =
    "Recording needs ffmpeg, which wasn't found. Reinstall rshot, or put ffmpeg.exe on your PATH.";

/// Why a new recording can't start now (no ffmpeg, one is running or still being saved); `None`
/// when it can.
pub fn unavailable(app: &AppHandle) -> Option<&'static str> {
    if ffmpeg_path().is_none() {
        Some(NO_FFMPEG)
    } else if is_recording(app) {
        Some("A recording is already running: stop it first.")
    } else if is_ending() {
        Some("The last recording is still being saved.")
    } else {
        None
    }
}

/// yuv420p needs even dimensions.
pub fn even(v: u32) -> u32 {
    v & !1
}

// The grabbers' argument builders and device-list parsers are compiled on every OS, so all of
// them are tested on Linux; each OS uses its own.

/// Parses `ffmpeg -sources pulse`, keeping inputs (monitor sources are dropped).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_pulse_sources(out: &str) -> Vec<Mic> {
    out.lines()
        .filter_map(|line| {
            let line = line.trim_start_matches(['*', ' ']);
            let (id, rest) = line.split_once(' ')?;
            let label = rest.split_once('[')?.1.rsplit_once(']')?.0;
            (!id.ends_with(".monitor")).then(|| Mic {
                id: id.into(),
                label: label.into(),
            })
        })
        .collect()
}

/// The mic meter's floor: silence (`-inf`) and garbage read as this.
pub const SILENCE_DB: f32 = -100.0;
const LEVEL_KEY: &str = "lavfi.astats.Overall.RMS_level";

/// One line of the level filter's stdout (`lavfi.astats.Overall.RMS_level=-21.06`) → dBFS.
fn parse_level(line: &str) -> Option<f32> {
    let db: f32 = line
        .strip_prefix(LEVEL_KEY)?
        .strip_prefix('=')?
        .trim()
        .parse()
        .ok()?;
    Some(if db.is_nan() {
        SILENCE_DB
    } else {
        db.clamp(SILENCE_DB, 0.0)
    })
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// What every grabber encodes with: H.264; with a mic, AAC and the level meter's filter; then the
/// crash-safe cluster flags and the output.
fn encode_tail(mic: bool, out: &Path) -> Vec<String> {
    let mut a = strs(&[
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p",
    ]);
    if mic {
        a.extend(strs(&["-c:a", "aac", "-b:a", "160k", "-af"]));
        // The mic level, 10 times a second, as `LEVEL_KEY=<dB>` lines on stdout (`direct`: unbuffered,
        // else they only arrive in 32 KiB bursts, about 40 s apart).
        a.push(format!(
            "asetnsamples=n=4800:p=0,astats=metadata=1:reset=1,ametadata=mode=print:key={LEVEL_KEY}:file='pipe\\:1':direct=1"
        ));
    }
    // A cluster (and a flush) every second, so a crash loses at most about a second.
    a.extend(strs(&[
        "-cluster_time_limit",
        "1000",
        "-flush_packets",
        "1",
    ]));
    a.push("-y".into());
    a.push(out.display().to_string());
    a
}

/// Linux: X11 `display`, and a PulseAudio/PipeWire source.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn x11grab_args(r: Region, display: &str, fps: u8, mic: Option<&str>, out: &Path) -> Vec<String> {
    let mut a = strs(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "x11grab",
        "-framerate",
    ]);
    a.push(fps.to_string());
    a.extend(strs(&["-draw_mouse", "1", "-video_size"]));
    a.push(format!("{}x{}", even(r.w), even(r.h)));
    a.push("-i".into());
    a.push(format!("{display}+{},{}", r.x, r.y));
    if let Some(m) = mic {
        a.extend(strs(&["-f", "pulse", "-i", m]));
    }
    a.extend(encode_tail(mic.is_some(), out));
    a
}

/// Windows: the desktop at `r` (physical px, negative left of the primary monitor), and a
/// DirectShow audio device by name.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn gdigrab_args(r: Region, fps: u8, mic: Option<&str>, out: &Path) -> Vec<String> {
    let mut a = strs(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "gdigrab",
        "-framerate",
    ]);
    a.push(fps.to_string());
    a.extend(strs(&["-draw_mouse", "1", "-offset_x"]));
    a.push(r.x.to_string());
    a.push("-offset_y".into());
    a.push(r.y.to_string());
    a.push("-video_size".into());
    a.push(format!("{}x{}", even(r.w), even(r.h)));
    a.extend(strs(&["-i", "desktop"]));
    if let Some(m) = mic {
        a.extend(strs(&["-f", "dshow", "-i"]));
        a.push(format!("audio={m}"));
    }
    a.extend(encode_tail(mic.is_some(), out));
    a
}

/// macOS: avfoundation video device `screen_dev` (a "Capture screen N") and audio device
/// `audio_dev`, both by index (a name would be prefix-matched, and one starting with a digit read
/// as an index), cropped to `crop`'s area (`lx`/`ly`/`w`/`h` of its `fw`×`fh` monitor image).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn avfoundation_args(
    screen_dev: usize,
    audio_dev: Option<usize>,
    crop: Option<Region>,
    fps: u8,
    out: &Path,
) -> Vec<String> {
    let mut a = strs(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "avfoundation",
        "-capture_cursor",
        "1",
        "-framerate",
    ]);
    a.push(fps.to_string());
    // A format the screen delivers: the default (yuv420p) is refused, with an error each time.
    a.extend(strs(&["-pixel_format", "uyvy422", "-i"]));
    let audio = audio_dev.map_or_else(|| "none".into(), |d| d.to_string());
    a.push(format!("{screen_dev}:{audio}"));
    if let Some(r) = crop {
        // In proportion to the monitor's image (`iw`×`ih` may be its pixels or its points), the
        // size rounded down to even.
        let (w, h, x, y, fw, fh) = (r.w, r.h, r.lx, r.ly, r.fw, r.fh);
        a.push("-vf".into());
        a.push(format!(
            "crop=trunc(iw*{w}/{fw}/2)*2:trunc(ih*{h}/{fh}/2)*2:trunc(iw*{x}/{fw}):trunc(ih*{y}/{fh})"
        ));
    }
    a.extend(encode_tail(audio_dev.is_some(), out));
    a
}

/// The audio devices in `ffmpeg -list_devices true -f dshow -i dummy`'s stderr, by name (the id
/// `-i audio=<name>` takes): `"<name>" (<types>)` lines whose types include audio, e.g.
/// `(audio)` or `(video, audio)`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_dshow_audio(out: &str) -> Vec<Mic> {
    out.lines()
        .filter_map(|l| {
            let (name, types) = l.split_once('"')?.1.split_once('"')?;
            let types = types.trim().strip_prefix('(')?.strip_suffix(')')?;
            types.split(',').any(|t| t.trim() == "audio").then(|| Mic {
                id: name.into(),
                label: name.into(),
            })
        })
        .collect()
}

/// An avfoundation device: (index, name).
type Device = (usize, String);

/// `ffmpeg -f avfoundation -list_devices true -i ""`'s stderr → the screens as (N of "Capture
/// screen N", its video device index), and the audio devices.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn parse_avfoundation(out: &str) -> (Vec<(usize, usize)>, Vec<Device>) {
    let (mut screens, mut mics, mut audio) = (vec![], vec![], false);
    for line in out.lines() {
        if line.contains("audio devices:") {
            audio = true;
            continue;
        }
        let Some((_, rest)) = line.split_once("] [") else {
            continue;
        };
        let Some((idx, name)) = rest.split_once("] ") else {
            continue;
        };
        let Ok(idx) = idx.parse::<usize>() else {
            continue;
        };
        if audio {
            mics.push((idx, name.trim().to_string()));
        } else if let Some(Ok(n)) = name.trim().strip_prefix("Capture screen ").map(str::parse) {
            screens.push((n, idx));
        }
    }
    (screens, mics)
}

/// The index of the mic named `name` (exactly) among the audio `devices`. macOS keeps the mic by
/// name, as indexes shift when devices come and go.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn audio_index(devices: &[Device], name: &str) -> Result<usize, String> {
    devices
        .iter()
        .find(|(_, n)| n == name)
        .map(|(i, _)| *i)
        .ok_or_else(|| format!("The microphone “{name}” isn't connected."))
}

/// ffmpeg's device list on macOS (on stderr; ffmpeg then fails, as there's no input).
#[cfg(target_os = "macos")]
const AVFOUNDATION_LIST: [&str; 7] = [
    "-hide_banner",
    "-f",
    "avfoundation",
    "-list_devices",
    "true",
    "-i",
    "",
];

/// This OS's grab of `r` (`full`: its whole monitor). On macOS it first runs `ffmpeg` (up to 5 s)
/// to find the screen's avfoundation device: call it before taking the recording lock.
pub fn record_args(
    ffmpeg: &Path,
    r: Region,
    display: &str,
    full: bool,
    fps: u8,
    mic: Option<&str>,
    out: &Path,
) -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let _ = (ffmpeg, full);
        Ok(x11grab_args(r, display, fps, mic, out))
    }
    #[cfg(target_os = "windows")]
    {
        let _ = (ffmpeg, display, full);
        Ok(gdigrab_args(r, fps, mic, out))
    }
    #[cfg(target_os = "macos")]
    {
        let _ = display;
        let mut cmd = ffmpeg_command(ffmpeg);
        cmd.args(AVFOUNDATION_LIST);
        let list = output_within(cmd, Duration::from_secs(5))?;
        let (screens, mics) = parse_avfoundation(&String::from_utf8_lossy(&list.stderr));
        let dev = screens
            .iter()
            .find(|(n, _)| *n == r.screen)
            .map(|(_, d)| *d)
            .ok_or("ffmpeg doesn't list this screen for recording")?;
        let audio = mic.map(|name| audio_index(&mics, name)).transpose()?;
        Ok(avfoundation_args(
            dev,
            audio,
            (!full).then_some(r),
            fps,
            out,
        ))
    }
}

pub fn remux_args(mkv: &Path, mp4: &Path) -> Vec<String> {
    let (mkv, mp4) = (mkv.display().to_string(), mp4.display().to_string());
    strs(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-i",
        &mkv,
        "-c",
        "copy",
        "-movflags",
        "+faststart",
        "-y",
        &mp4,
    ])
}

/// Frame-accurate: `-ss` before `-i` with a re-encode seeks exactly. Muted drops the audio. An empty
/// result fails (`-abort_on empty_output`), so it can never be renamed over the recording.
pub fn trim_args(src: &Path, start: f64, end: f64, mute: bool, out: &Path) -> Vec<String> {
    let (src, out) = (src.display().to_string(), out.display().to_string());
    let (ss, t) = (format!("{start:.3}"), format!("{:.3}", end - start));
    let mut a = strs(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-abort_on",
        "empty_output",
        "-ss",
        &ss,
        "-i",
        &src,
        "-t",
        &t,
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
    ]);
    a.extend(strs(if mute { &["-an"] } else { &["-c:a", "aac"] }));
    a.extend(strs(&["-movflags", "+faststart", "-y", &out]));
    a
}

/// The first frame as one PNG on stdout, at most `width` px wide. At the default log level, ffmpeg
/// also prints the input's `Duration:` line to stderr (`parse_duration`).
pub fn poster_args(src: &Path, width: u32) -> Vec<String> {
    let (src, scale) = (
        src.display().to_string(),
        format!("scale='min({width},iw)':-1"),
    );
    strs(&[
        "-hide_banner",
        "-nostats",
        "-i",
        &src,
        "-frames:v",
        "1",
        "-vf",
        &scale,
        "-f",
        "image2pipe",
        "-c:v",
        "png",
        "-",
    ])
}

/// Seconds from the `  Duration: 00:01:02.50, start: …` line of ffmpeg's stderr; `None` for `N/A`.
/// Only a line that starts with it counts: a title can contain "Duration: " too.
pub fn parse_duration(stderr: &str) -> Option<f64> {
    let line = stderr
        .lines()
        .find_map(|l| l.trim_start().strip_prefix("Duration: "))?;
    let hms = line.split(',').next()?;
    let mut parts = hms.split(':').map(|p| p.trim().parse::<f64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Runs `cmd` to completion like `Command::output`, but kills it after `limit`.
pub fn output_within(mut cmd: Command, limit: Duration) -> Result<std::process::Output, String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(err)?;
    // Both pipes drain on their own threads, so a chatty child can't block on a full pipe.
    let drain = |mut r: Box<dyn std::io::Read + Send>| {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = r.read_to_end(&mut b);
            b
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("piped")));
    let errs = drain(Box::new(child.stderr.take().expect("piped")));
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(err)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("ffmpeg took too long".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Ok(std::process::Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: errs.join().unwrap_or_default(),
    })
}

static GENERATION: AtomicU64 = AtomicU64::new(1);

pub struct Recording {
    id: u64,
    child: Child,
    ffmpeg: PathBuf,
    mkv: PathBuf,
    mp4: PathBuf,
    started: Instant,
    /// Latest mic level (f32 dB bits); `None` without a mic.
    level: Option<Arc<AtomicU32>>,
}

pub fn is_recording(app: &AppHandle) -> bool {
    app.state::<AppState>().recording.lock().unwrap().is_some()
}

fn ffmpeg_log_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("rshot")
        .join("ffmpeg.log")
}

/// ffmpeg's own log, so a failed recording can be diagnosed: fresh per recording, and the remux
/// and trims append to it.
pub(crate) fn ffmpeg_log(append: bool) -> Stdio {
    let path = ffmpeg_log_path();
    let _ = std::fs::create_dir_all(path.parent().expect("has a parent"));
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null())
}

/// Where to look after a failure: the raw file, if ffmpeg got as far as writing one, and the log.
fn leftovers(mkv: &Path) -> String {
    let log = ffmpeg_log_path();
    if mkv.exists() {
        format!(
            "The raw file is kept at {}. ffmpeg's log: {}",
            mkv.display(),
            log.display()
        )
    } else {
        format!("ffmpeg's log: {}", log.display())
    }
}

pub fn start(app: &AppHandle, region: Region, full: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();
    let ffmpeg = ffmpeg_path().ok_or(NO_FFMPEG)?;
    // Picked before the lock, since the arguments name it: two racing starts may pick the same
    // name, but only one of them gets the slot and spawns.
    let mp4 = store::new_recording_path(&cfg).map_err(err)?;
    let mkv = store::raw_recording(&mp4);
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
    let fps = if cfg.fps == 60 { 60 } else { 30 };
    // Not under the lock: on macOS this runs ffmpeg to find the screen (up to 5 s), which would
    // block `is_recording` and `recording_info` on the main thread meanwhile.
    let args = record_args(
        &ffmpeg,
        region,
        &display,
        full,
        fps,
        cfg.mic.as_deref(),
        &mkv,
    )?;
    // Checked, spawned and stored under one lock: two racing starts can't both spawn an ffmpeg.
    let id = {
        let mut slot = state.recording.lock().unwrap();
        if slot.is_some() {
            return Err("Already recording".into());
        }
        let mut cmd = ffmpeg_command(&ffmpeg);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(ffmpeg_log(false));
        let level = cfg
            .mic
            .is_some()
            .then(|| Arc::new(AtomicU32::new(SILENCE_DB.to_bits())));
        let child = spawn_tied(cmd, level.clone())?;
        let id = GENERATION.fetch_add(1, Ordering::Relaxed);
        *slot = Some(Recording {
            id,
            child,
            ffmpeg,
            mkv,
            mp4,
            started: Instant::now(),
            level,
        });
        id
    };
    watch(app.clone(), id);
    // ffmpeg is running and watched: missing UI is worth a notification, not a failed start.
    if let Err(e) = ui::recording_started(app, region, full) {
        pipeline::notify(app, &e);
    }
    // A stop that landed while the UI came up reset it before we set it: reset it again. (A newer
    // recording in the slot shows the same "recording" UI, so it's left alone.)
    if !is_recording(app) {
        ui::recording_stopped(app);
    }
    Ok(())
}

/// Spawns ffmpeg from a thread that lives exactly as long as ffmpeg (it drains ffmpeg's stdout to
/// EOF, feeding `level` when set). A killed rshot must not leave the screen being recorded:
/// - Linux: ffmpeg gets SIGTERM when that thread ends, which includes rshot dying. (The parent
///   death signal is per thread, so it can't come from a short-lived caller thread.) When rshot is
///   killed outright, the kernel re-sends the signal each time ffmpeg is reparented to a
///   still-dying thread, so ffmpeg hard-exits (> 3 signals): the MKV keeps what was flushed, as
///   after a crash.
/// - Windows: ffmpeg joins a job object that kills it when rshot's handle to it closes, i.e. when
///   rshot exits, however it ends.
///
/// ponytail: macOS has neither, so a killed rshot (not a Quit, which stops the recording) leaves
/// ffmpeg recording until it's killed or the disk is full. Upgrade: a tiny parent-watch in the
/// child (kqueue EVFILT_PROC NOTE_EXIT on rshot's pid) that sends ffmpeg `q`.
fn spawn_tied(mut cmd: Command, level: Option<Arc<AtomicU32>>) -> Result<Child, String> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: prctl is async-signal-safe, as pre_exec requires.
        unsafe {
            cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    std::thread::spawn(move || {
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return drop(tx.send(Err(format!("ffmpeg: {e}")))),
        };
        #[cfg(windows)]
        if let Err(e) = kill_with_rshot(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return drop(tx.send(Err(format!("ffmpeg: {e}"))));
        }
        let out = child.stdout.take().expect("stdout is piped");
        if tx.send(Ok(child)).is_err() {
            return;
        }
        // To EOF whatever it reads: this thread ending SIGTERMs ffmpeg. A bad line is skipped.
        let mut reader = BufReader::new(out);
        let mut line = Vec::new();
        while reader.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
            let db = std::str::from_utf8(&line).ok().and_then(parse_level);
            if let (Some(level), Some(db)) = (&level, db) {
                level.store(db.to_bits(), Ordering::Relaxed);
            }
            line.clear();
        }
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
    });
    rx.recv().map_err(err)?
}

/// Windows: puts `child` in rshot's job, which kills its processes once its only handle (never
/// closed, so it closes as rshot exits) goes. Fails, like a failed spawn, when it can't.
#[cfg(windows)]
fn kill_with_rshot(child: &Child) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
    };
    // Made on first use and kept once it works (a failure is retried by the next recording).
    // The handle as an integer: a raw pointer isn't Send, so it can't sit in a static.
    static JOB: std::sync::Mutex<Option<isize>> = std::sync::Mutex::new(None);
    let mut kept = JOB
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let job = match *kept {
        Some(job) => HANDLE(job as *mut std::ffi::c_void),
        None => {
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: a fresh unnamed job; `limits` outlives the call, which reads its exact size.
            // A job that can't take the limit is closed, not kept.
            unsafe {
                let job = CreateJobObjectW(None, windows::core::PCWSTR::null()).map_err(err)?;
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    std::ptr::from_ref(&limits).cast(),
                    std::mem::size_of_val(&limits) as u32,
                )
                .inspect_err(|_| {
                    let _ = CloseHandle(job);
                })
                .map_err(err)?;
                *kept = Some(job.0 as isize);
                job
            }
        }
    };
    // SAFETY: both handles are open: the job's for rshot's lifetime, the child's while `child` is.
    unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())) }.map_err(err)
}

/// Ticks the tray timer; if ffmpeg dies on its own, keeps the MKV and tells the user.
fn watch(app: AppHandle, id: u64) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        let state = app.state::<AppState>();
        let mut guard = state.recording.lock().unwrap();
        let Some(rec) = guard.as_mut().filter(|r| r.id == id) else {
            // Stopped: the last tick may have re-set the timer just after the stop cleared it.
            if guard.is_none() {
                drop(guard);
                ui::set_tray_timer(&app, None);
            }
            return;
        };
        if matches!(rec.child.try_wait(), Ok(Some(_))) {
            let rec = guard.take().expect("checked above");
            drop(guard);
            ui::recording_stopped(&app);
            pipeline::notify(
                &app,
                &format!("Recording stopped unexpectedly. {}", leftovers(&rec.mkv)),
            );
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

/// Held while a stop or a discard finishes a recording, which takes it out of the slot first:
/// Quit waits on it, so it can't exit halfway through a remux.
static ENDING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn ending() -> std::sync::MutexGuard<'static, ()> {
    ENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A stop or a discard is finishing a recording.
pub fn is_ending() -> bool {
    matches!(ENDING.try_lock(), Err(std::sync::TryLockError::WouldBlock))
}

/// Asks ffmpeg to finish (`q`), waits up to `within`, then kills it.
fn end(rec: &mut Recording, within: Duration) {
    if let Some(mut stdin) = rec.child.stdin.take() {
        let _ = stdin.write_all(b"q\n");
    }
    let deadline = Instant::now() + within;
    while matches!(rec.child.try_wait(), Ok(None)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = rec.child.kill();
    let _ = rec.child.wait();
}

/// Remuxes the raw `mkv` into `mp4`, atomically (a hidden `.part.mp4`, then renamed); the MKV,
/// which reserves the name, goes last. On failure the MKV stays.
fn finalize(ffmpeg: &Path, mkv: &Path, mp4: &Path) -> Result<(), String> {
    let part = mkv.with_extension("part.mp4");
    ffmpeg_command(ffmpeg)
        .args(remux_args(mkv, &part))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(ffmpeg_log(true))
        .status()
        .map_err(err)
        .and_then(|s| s.success().then_some(()).ok_or(s.to_string()))
        .and_then(|()| std::fs::rename(&part, mp4).map_err(err))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&part);
        })?;
    let _ = std::fs::remove_file(mkv);
    Ok(())
}

/// Ends the recording and turns it into the MP4. Nothing to do when none is running, e.g. the
/// watchdog already ended it. Blocks for up to ~10 s (and waits for a stop or a discard already
/// under way): not on the main thread.
pub fn stop(app: &AppHandle) -> Result<(), String> {
    let _ending = ending();
    let Some(mut rec) = take(app) else {
        return Ok(());
    };
    end(&mut rec, Duration::from_secs(10));
    finalize(&rec.ffmpeg, &rec.mkv, &rec.mp4).map_err(|e| {
        format!(
            "Couldn't finish the recording ({e}). {}",
            leftovers(&rec.mkv)
        )
    })?;
    pipeline::finish_video(app, &rec.mp4)
}

/// rshot is exiting without a Quit, which would have stopped the recording (macOS ⌘Q or logout,
/// Windows shutdown or sign-out): ffmpeg gets ~2 s to close the MKV, which the next launch's
/// `recover` turns into the MP4. On the main thread, so a stop under way (ENDING) isn't waited for.
#[cfg(not(target_os = "linux"))]
pub fn end_at_exit(app: &AppHandle) {
    // Never panics: a panic in the exit callback would abort rshot before ffmpeg is stopped.
    let rec = app
        .state::<AppState>()
        .recording
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(mut rec) = rec {
        end(&mut rec, Duration::from_secs(2));
    }
}

/// At startup: a recording no rshot finished (logout, shutdown, crash, kill) left its raw MKV in
/// the recordings folder. Each becomes its MP4, and the user is told where; a stop's or a trim's
/// stale temp goes. Not on the main thread.
pub fn recover(app: &AppHandle) {
    let state = app.state::<AppState>();
    let dir = store::recordings_dir(&state.config.lock().unwrap());
    // Listed under the recording lock, with none running or ending: one started afterwards gets a
    // name that's free, so its MKV can't be in the list.
    let names: Vec<String> = {
        let slot = state.recording.lock().unwrap();
        if slot.is_some() || is_ending() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        entries
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect()
    };
    for name in names.iter().filter(|n| store::stale_temp(n)) {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let Some(ffmpeg) = ffmpeg_path() else {
        return;
    };
    for (name, stem) in names
        .iter()
        .filter_map(|n| Some((n, store::orphan_stem(n)?)))
    {
        let mkv = dir.join(name);
        // Written in the last 5 s: its ffmpeg may still be finishing after its SIGTERM. Next start.
        let fresh = mkv
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|d| d < Duration::from_secs(5));
        if fresh {
            continue;
        }
        let mp4 = store::unused(&dir, stem, "mp4");
        let msg = match finalize(&ffmpeg, &mkv, &mp4) {
            Ok(()) => format!("Recovered an interrupted recording: {}", mp4.display()),
            Err(e) => {
                // Kept under a visible name, so it isn't retried (and notified) at every start.
                let kept = store::unused(&dir, stem, "mkv");
                let mkv = if std::fs::rename(&mkv, &kept).is_ok() {
                    kept
                } else {
                    mkv
                };
                format!(
                    "Couldn't recover an interrupted recording ({e}). {}",
                    leftovers(&mkv)
                )
            }
        };
        pipeline::notify(app, &msg);
    }
}

pub fn discard(app: &AppHandle) {
    let _ending = ending();
    if let Some(mut rec) = take(app) {
        end(&mut rec, Duration::from_secs(10));
        let _ = std::fs::remove_file(&rec.mkv);
    }
}

#[derive(Serialize)]
pub struct RecordingInfo {
    elapsed_ms: u64,
    mic: bool,
    /// dBFS, `SILENCE_DB`..=0; `None` without a mic.
    level: Option<f32>,
}

#[tauri::command]
pub fn recording_info(state: State<'_, AppState>) -> Option<RecordingInfo> {
    state
        .recording
        .lock()
        .unwrap()
        .as_ref()
        .map(|r| RecordingInfo {
            elapsed_ms: r.started.elapsed().as_millis() as u64,
            mic: r.level.is_some(),
            level: r
                .level
                .as_ref()
                .map(|l| f32::from_bits(l.load(Ordering::Relaxed))),
        })
}

// Stop and discard can wait ~10 s for ffmpeg, and listing the mics runs it: on a blocking thread,
// neither on the main thread nor on an async worker.

#[tauri::command]
pub async fn recording_stop(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        stop(&app).inspect_err(|e| pipeline::notify(&app, e))
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn recording_discard(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || discard(&app)).await;
}

#[tauri::command]
pub async fn list_mics() -> Vec<Mic> {
    tauri::async_runtime::spawn_blocking(|| {
        let Some(ffmpeg) = ffmpeg_path() else {
            return Vec::new();
        };
        let mut cmd = ffmpeg_command(&ffmpeg);
        #[cfg(target_os = "linux")]
        cmd.args(["-hide_banner", "-sources", "pulse"]);
        #[cfg(windows)]
        cmd.args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ]);
        #[cfg(target_os = "macos")]
        cmd.args(AVFOUNDATION_LIST);
        let Ok(o) = output_within(cmd, Duration::from_secs(5)) else {
            return Vec::new();
        };
        // Windows and macOS list on stderr (then ffmpeg fails: there's no input).
        #[cfg(target_os = "linux")]
        {
            parse_pulse_sources(&String::from_utf8_lossy(&o.stdout))
        }
        #[cfg(windows)]
        {
            parse_dshow_audio(&String::from_utf8_lossy(&o.stderr))
        }
        #[cfg(target_os = "macos")]
        {
            parse_avfoundation(&String::from_utf8_lossy(&o.stderr))
                .1
                .into_iter()
                .map(|(_, name)| Mic {
                    id: name.clone(),
                    label: name,
                })
                .collect()
        }
    })
    .await
    .unwrap_or_default()
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
        let out = "Auto-detected sources for pulse:\n  alsa_output.x.monitor [Monitor of X] (none)\n* alsa_input.usb-mic.mono [USB Mic] (none)\n  alsa_input.pci.analog-stereo [Built-in Audio [Rear]] (none)\n";
        assert_eq!(
            parse_pulse_sources(out),
            vec![
                Mic {
                    id: "alsa_input.usb-mic.mono".into(),
                    label: "USB Mic".into()
                },
                Mic {
                    id: "alsa_input.pci.analog-stereo".into(),
                    label: "Built-in Audio [Rear]".into()
                },
            ]
        );
    }

    #[test]
    fn record_args_grab_x11_with_optional_mic() {
        let r = Region {
            x: 1920,
            y: 10,
            w: 871,
            h: 569,
            screen: 0,
            lx: 0,
            ly: 0,
            fw: 0,
            fh: 0,
        };
        let a = x11grab_args(r, ":1", 30, None, Path::new("/v/.R.mkv"));
        assert_eq!(
            a,
            s(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "x11grab",
                "-framerate",
                "30",
                "-draw_mouse",
                "1",
                "-video_size",
                "870x568",
                "-i",
                ":1+1920,10",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "23",
                "-pix_fmt",
                "yuv420p",
                "-cluster_time_limit",
                "1000",
                "-flush_packets",
                "1",
                "-y",
                "/v/.R.mkv",
            ])
        );
        let m = x11grab_args(r, ":1", 60, Some("alsa_input.usb"), Path::new("/v/.R.mkv"));
        assert_eq!(
            m,
            s(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "x11grab",
                "-framerate",
                "60",
                "-draw_mouse",
                "1",
                "-video_size",
                "870x568",
                "-i",
                ":1+1920,10",
                "-f",
                "pulse",
                "-i",
                "alsa_input.usb",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "23",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-af",
                "asetnsamples=n=4800:p=0,astats=metadata=1:reset=1,ametadata=mode=print:key=lavfi.astats.Overall.RMS_level:file='pipe\\:1':direct=1",
                "-cluster_time_limit",
                "1000",
                "-flush_packets",
                "1",
                "-y",
                "/v/.R.mkv",
            ])
        );
    }

    /// The encode tail every grabber shares: H.264, then with a mic AAC and the level meter, then
    /// the crash-safe cluster flags and the output.
    fn tail(mic: bool, out: &str) -> Vec<String> {
        let mut t = s(&[
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p",
        ]);
        if mic {
            t.extend(s(&[
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-af",
                "asetnsamples=n=4800:p=0,astats=metadata=1:reset=1,ametadata=mode=print:key=lavfi.astats.Overall.RMS_level:file='pipe\\:1':direct=1",
            ]));
        }
        t.extend(s(&[
            "-cluster_time_limit",
            "1000",
            "-flush_packets",
            "1",
            "-y",
            out,
        ]));
        t
    }

    #[test]
    fn gdigrab_args_offset_and_mic() {
        let r = Region {
            x: -1920,
            y: 0,
            w: 1281,
            h: 720,
            screen: 0,
            lx: 0,
            ly: 0,
            fw: 0,
            fh: 0,
        };
        let a = gdigrab_args(
            r,
            30,
            Some("Microphone (Realtek)"),
            Path::new("C:/v/.R.mkv"),
        );
        assert!(a
            .windows(4)
            .any(|w| w == s(&["-offset_x", "-1920", "-offset_y", "0"])));
        assert!(a.windows(2).any(|w| w == s(&["-video_size", "1280x720"])));
        assert!(a
            .windows(4)
            .any(|w| w == s(&["-f", "dshow", "-i", "audio=Microphone (Realtek)"])));
        assert!(a.windows(2).any(|w| w == s(&["-i", "desktop"])));
        assert!(a.ends_with(&tail(true, "C:/v/.R.mkv")));
        let b = gdigrab_args(r, 60, None, Path::new("C:/v/.R.mkv"));
        assert!(!b.iter().any(|x| x == "dshow"));
        assert!(b.ends_with(&tail(false, "C:/v/.R.mkv")));
    }

    #[test]
    fn parses_dshow_audio_devices() {
        let out = "[dshow @ 000001] \"Integrated Camera\" (video)\n[dshow @ 000001]   Alternative name \"@device_pnp_x\"\n[dshow @ 000001] \"Microphone (Realtek(R) Audio)\" (audio)\n[dshow @ 000001]   Alternative name \"@device_cm_y\"\n";
        assert_eq!(
            parse_dshow_audio(out),
            vec![Mic {
                id: "Microphone (Realtek(R) Audio)".into(),
                label: "Microphone (Realtek(R) Audio)".into()
            }]
        );
        // CRLF line ends; a device with several pin types (a capture card) lists them all; the
        // listing ends with ffmpeg's error for the input it then refuses to open.
        let more = "[dshow @ 0000021a] \"HD60 S+\" (video, audio)\r\n[dshow @ 0000021a] \"Mikrofon (USB Audio)\" (audio)\r\n[dshow @ 0000021a]   Alternative name \"@device_cm_{33D9}\"\r\n[dshow @ 0000021a] \"OBS Virtual Camera\" (video)\r\n[in#0 @ 0000021b] Error opening input: Immediate exit requested\r\n";
        let names: Vec<String> = parse_dshow_audio(more).into_iter().map(|m| m.id).collect();
        assert_eq!(names, s(&["HD60 S+", "Mikrofon (USB Audio)"]));
    }

    #[test]
    fn parses_avfoundation_devices() {
        let out = "[AVFoundation indev @ 0x1] AVFoundation video devices:\n[AVFoundation indev @ 0x1] [0] FaceTime HD Camera\n[AVFoundation indev @ 0x1] [1] Capture screen 0\n[AVFoundation indev @ 0x1] [2] Capture screen 1\n[AVFoundation indev @ 0x1] AVFoundation audio devices:\n[AVFoundation indev @ 0x1] [0] MacBook Pro Microphone\n";
        let (screens, mics) = parse_avfoundation(out);
        assert_eq!(screens, vec![(0, 1), (1, 2)]);
        assert_eq!(mics, vec![(0, "MacBook Pro Microphone".to_string())]);
        // The saved name → today's index; exact, never a prefix; a missing one is an error.
        let devices = [
            (0, "MacBook Pro Microphone".to_string()),
            (1, "MacBook".to_string()),
        ];
        assert_eq!(audio_index(&devices, "MacBook"), Ok(1));
        assert_eq!(audio_index(&devices, "MacBook Pro Microphone"), Ok(0));
        assert_eq!(
            audio_index(&devices, "USB Mic"),
            Err("The microphone “USB Mic” isn't connected.".to_string())
        );
        assert_eq!(
            parse_avfoundation("[in#0 @ 0x2] Error opening input: Input/output error\n"),
            (vec![], vec![])
        );
    }

    #[test]
    fn avfoundation_args_crop_in_proportion() {
        // 641×480 at (10, 20) of a 2880×1800 image: the same area whether the screen's frames
        // come in pixels (iw 2880: 640×480 at 10, 20) or in points (iw 1440: 320×240 at 5, 10).
        let r = Region {
            x: 1450,
            y: 20,
            w: 641,
            h: 480,
            screen: 1,
            lx: 10,
            ly: 20,
            fw: 2880,
            fh: 1800,
        };
        let a = avfoundation_args(1, Some(0), Some(r), 60, Path::new("/v/.R.mkv"));
        let mut want = s(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "avfoundation",
            "-capture_cursor",
            "1",
            "-framerate",
            "60",
            "-pixel_format",
            "uyvy422",
            "-i",
            "1:0",
            "-vf",
            "crop=trunc(iw*641/2880/2)*2:trunc(ih*480/1800/2)*2:trunc(iw*10/2880):trunc(ih*20/1800)",
        ]);
        want.extend(tail(true, "/v/.R.mkv"));
        assert_eq!(a, want);
        let b = avfoundation_args(1, None, None, 30, Path::new("/o.mkv"));
        assert!(b.windows(2).any(|w| w == s(&["-i", "1:none"])));
        assert!(b
            .windows(3)
            .any(|w| w == s(&["-pixel_format", "uyvy422", "-i"])));
        assert!(!b.iter().any(|x| x == "-vf"));
        assert!(b.ends_with(&tail(false, "/o.mkv")));
    }

    #[test]
    fn mic_level_lines_parse_to_clamped_db() {
        assert_eq!(
            parse_level("lavfi.astats.Overall.RMS_level=-21.5"),
            Some(-21.5)
        );
        assert_eq!(
            parse_level("lavfi.astats.Overall.RMS_level=-inf"),
            Some(SILENCE_DB)
        );
        assert_eq!(
            parse_level("lavfi.astats.Overall.RMS_level=nan"),
            Some(SILENCE_DB)
        );
        assert_eq!(parse_level("lavfi.astats.Overall.RMS_level=3.5"), Some(0.0));
        // As the drain reads it: with the newline.
        assert_eq!(
            parse_level("lavfi.astats.Overall.RMS_level=-21.5\n"),
            Some(-21.5)
        );
        assert_eq!(parse_level("frame:1    pts:4800    pts_time:0.1"), None);
        assert_eq!(parse_level("lavfi.astats.Overall.RMS_level=junk"), None);
    }

    /// `cargo test recorder -- --ignored` on a desktop session: records 1 s of the top-left 320×240 and remuxes it.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn records_and_remuxes_one_second() {
        let ffmpeg = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/rshot-ffmpeg-x86_64-unknown-linux-gnu");
        let dir = std::env::temp_dir().join(format!("rshot-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mkv, mp4) = (dir.join(".R.mkv"), dir.join("R.mp4"));
        let display = std::env::var("DISPLAY").unwrap();
        let region = Region {
            x: 0,
            y: 0,
            w: 320,
            h: 240,
            screen: 0,
            lx: 0,
            ly: 0,
            fw: 0,
            fh: 0,
        };
        let mut args = record_args(&ffmpeg, region, &display, false, 30, None, &mkv).unwrap();
        args.splice(args.len() - 2..args.len() - 2, strs(&["-t", "1"]));
        let run = |args: &[String]| {
            ffmpeg_command(&ffmpeg)
                .args(args)
                .status()
                .unwrap()
                .success()
        };
        let ok = run(&args) && run(&remux_args(&mkv, &mp4));
        let len = std::fs::metadata(&mp4).map(|m| m.len()).unwrap_or(0);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(ok && len > 1000, "ok={ok} len={len}");
    }

    #[test]
    fn remux_copies_streams() {
        assert_eq!(
            remux_args(Path::new("/v/.R.mkv"), Path::new("/v/R.mp4")),
            s(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                "/v/.R.mkv",
                "-c",
                "copy",
                "-movflags",
                "+faststart",
                "-y",
                "/v/R.mp4"
            ])
        );
    }

    #[test]
    fn trim_reencodes_the_kept_range() {
        let a = trim_args(
            Path::new("/v/R.mp4"),
            4.8,
            28.6,
            true,
            Path::new("/v/.R.trim.mp4"),
        );
        assert_eq!(
            a,
            s(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-abort_on",
                "empty_output",
                "-ss",
                "4.800",
                "-i",
                "/v/R.mp4",
                "-t",
                "23.800",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "20",
                "-pix_fmt",
                "yuv420p",
                "-an",
                "-movflags",
                "+faststart",
                "-y",
                "/v/.R.trim.mp4",
            ])
        );
        let b = trim_args(Path::new("/v/R.mp4"), 0.0, 1.0, false, Path::new("/o.mp4"));
        assert!(b.windows(2).any(|w| w == s(&["-c:a", "aac"])));
        assert!(!b.contains(&"-an".to_string()));
    }

    #[test]
    fn poster_is_one_png_frame_on_stdout() {
        assert_eq!(
            poster_args(Path::new("/v/R.mp4"), 460),
            s(&[
                "-hide_banner",
                "-nostats",
                "-i",
                "/v/R.mp4",
                "-frames:v",
                "1",
                "-vf",
                "scale='min(460,iw)':-1",
                "-f",
                "image2pipe",
                "-c:v",
                "png",
                "-",
            ])
        );
    }

    #[cfg(unix)]
    #[test]
    fn output_within_kills_a_slow_command() {
        let t = Instant::now();
        let mut slow = Command::new("sleep");
        slow.arg("5");
        assert!(output_within(slow, Duration::from_millis(200)).is_err());
        assert!(t.elapsed() < Duration::from_secs(2));
        let mut quick = Command::new("echo");
        quick.arg("hi");
        let out = output_within(quick, Duration::from_secs(5)).unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout, b"hi\n");
    }

    #[test]
    fn duration_parses_from_ffmpeg_stderr() {
        let err = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '/v/R.mp4':\n  Metadata:\n    encoder         : Lavf61.7.100\n  Duration: 00:01:02.50, start: 0.000000, bitrate: 97 kb/s\n";
        assert_eq!(parse_duration(err), Some(62.5));
        assert_eq!(
            parse_duration("  Duration: 01:00:00.04, start: 0"),
            Some(3600.04)
        );
        assert_eq!(parse_duration("  Duration: N/A, start: 0"), None);
        assert_eq!(parse_duration("no input"), None);
        let decoy = "  Metadata:\n    title           : Duration: 09:09:09.00, fake\n  Duration: 00:00:20.20, start: 0.000000, bitrate: 712 kb/s\n";
        assert_eq!(parse_duration(decoy), Some(20.2));
    }
}
