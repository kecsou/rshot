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
    let name = if cfg!(windows) {
        "rshot-ffmpeg.exe"
    } else {
        "rshot-ffmpeg"
    };
    let bundled = std::env::current_exe().ok()?.parent()?.join(name);
    if bundled.is_file() {
        return Some(bundled);
    }
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(exe))
        .find(|p| p.is_file())
}

/// yuv420p needs even dimensions.
#[cfg(target_os = "linux")]
pub fn even(v: u32) -> u32 {
    v & !1
}

/// Parses `ffmpeg -sources pulse`, keeping inputs (monitor sources are dropped).
pub fn parse_pulse_sources(out: &str) -> Vec<Mic> {
    out.lines()
        .filter_map(|line| {
            let line = line.trim_start_matches(['*', ' ']);
            let (id, rest) = line.split_once(' ')?;
            let label = rest.split_once('[')?.1.split_once(']')?.0;
            (!id.ends_with(".monitor")).then(|| Mic {
                id: id.into(),
                label: label.into(),
            })
        })
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[cfg(target_os = "linux")]
pub fn record_args(
    r: Region,
    display: &str,
    fps: u8,
    mic: Option<&str>,
    out: &Path,
) -> Result<Vec<String>, String> {
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
    a.extend(strs(&[
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p",
    ]));
    if mic.is_some() {
        a.extend(strs(&["-c:a", "aac", "-b:a", "160k"]));
    }
    a.push("-y".into());
    a.push(out.display().to_string());
    Ok(a)
}

#[cfg(not(target_os = "linux"))]
pub fn record_args(
    _r: Region,
    _display: &str,
    _fps: u8,
    _mic: Option<&str>,
    _out: &Path,
) -> Result<Vec<String>, String> {
    Err("Recording on this OS arrives in a later version.".into())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[cfg(target_os = "linux")]
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
                Mic {
                    id: "alsa_input.usb-mic.mono".into(),
                    label: "USB Mic".into()
                },
                Mic {
                    id: "alsa_input.pci.analog-stereo".into(),
                    label: "Built-in Audio".into()
                },
            ]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn record_args_grab_x11_with_optional_mic() {
        let r = Region {
            x: 1920,
            y: 10,
            w: 871,
            h: 569,
        };
        let a = record_args(r, ":1", 30, None, Path::new("/v/.R.mkv")).unwrap();
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
                "-y",
                "/v/.R.mkv",
            ])
        );
        let m = record_args(r, ":1", 60, Some("alsa_input.usb"), Path::new("/v/.R.mkv")).unwrap();
        assert!(m
            .windows(4)
            .any(|w| w == s(&["-f", "pulse", "-i", "alsa_input.usb"])));
        assert!(m
            .windows(4)
            .any(|w| w == s(&["-c:a", "aac", "-b:a", "160k"])));
        assert!(m.windows(2).any(|w| w == s(&["-framerate", "60"])));
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
}
