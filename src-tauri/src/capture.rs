//! Screen grabbing (xcap), cropping, PNG encoding, window list, cursor compositing.

use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder, RgbaImage};
use serde::Serialize;

/// One monitor's frozen image. `x`/`y` are the monitor origin in physical desktop pixels.
pub struct Frame {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub image: RgbaImage,
}

/// A window, in pixels relative to the frame it was listed for.
#[derive(Serialize, Clone, Debug)]
pub struct WinRect {
    pub id: u32,
    pub title: String,
    pub app: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

use crate::err;

/// Grabs every monitor, on Linux each on its own thread (xcap opens a connection per call, so the
/// grabs and pixel conversions overlap). Frames keep `Monitor::all()` order.
pub fn grab_all(show_pointer: bool) -> Result<Vec<Frame>, String> {
    let cursor = if show_pointer { cursor::grab() } else { None };
    let monitors = xcap::Monitor::all().map_err(err)?;
    let grab = |m: &xcap::Monitor| -> Result<Frame, String> {
        let mut image = m.capture_image().map_err(err)?;
        let (x, y) = (m.x().map_err(err)?, m.y().map_err(err)?);
        if let Some(c) = &cursor {
            cursor::composite(&mut image, c, x, y);
        }
        Ok(Frame {
            name: m.name().unwrap_or_default(),
            x,
            y,
            image,
        })
    };
    #[cfg(target_os = "linux")]
    {
        std::thread::scope(|s| {
            let jobs: Vec<_> = monitors.iter().map(|m| s.spawn(move || grab(m))).collect();
            jobs.into_iter()
                .map(|j| j.join().map_err(|_| "monitor grab panicked".to_string())?)
                .collect()
        })
    }
    // ponytail: xcap's Windows monitor handle isn't Send, so other OSes grab one after another.
    #[cfg(not(target_os = "linux"))]
    {
        monitors.iter().map(grab).collect()
    }
}

/// Index of the frame containing the desktop point, or 0.
pub fn frame_at(frames: &[Frame], px: i32, py: i32) -> usize {
    frames
        .iter()
        .position(|f| {
            px >= f.x
                && py >= f.y
                && px < f.x + f.image.width() as i32
                && py < f.y + f.image.height() as i32
        })
        .unwrap_or(0)
}

/// Rounds and clips an image-pixel rect to the frame; `None` when nothing is left.
pub fn clamp_rect(x: f64, y: f64, w: f64, h: f64, fw: u32, fh: u32) -> Option<[u32; 4]> {
    let (fw, fh) = (fw as f64, fh as f64);
    let x0 = x.round().clamp(0.0, fw);
    let y0 = y.round().clamp(0.0, fh);
    let x1 = (x + w).round().clamp(0.0, fw);
    let y1 = (y + h).round().clamp(0.0, fh);
    (x1 > x0 && y1 > y0).then_some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}

pub fn crop(img: &RgbaImage, r: [u32; 4]) -> RgbaImage {
    image::imageops::crop_imm(img, r[0], r[1], r[2], r[3]).to_image()
}

pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(err)?;
    Ok(out)
}

/// Visible windows that intersect the frame, top-most first (xcap lists top → bottom).
pub fn windows_on(f: &Frame) -> Vec<WinRect> {
    let me = std::process::id();
    let (fw, fh) = (f.image.width() as i32, f.image.height() as i32);
    xcap::Window::all()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|w| {
            if w.pid().ok()? == me || w.is_minimized().unwrap_or(true) {
                return None;
            }
            let r = WinRect {
                id: w.id().ok()?,
                title: w.title().unwrap_or_default(),
                app: w.app_name().unwrap_or_default(),
                x: w.x().ok()? - f.x,
                y: w.y().ok()? - f.y,
                w: w.width().ok()?,
                h: w.height().ok()?,
            };
            let hits = r.w > 0
                && r.h > 0
                && r.x < fw
                && r.y < fh
                && r.x + r.w as i32 > 0
                && r.y + r.h as i32 > 0;
            hits.then_some(r)
        })
        .collect()
}

pub fn window_image(id: u32) -> Result<RgbaImage, String> {
    let w = xcap::Window::all()
        .map_err(err)?
        .into_iter()
        .find(|w| w.id().ok() == Some(id))
        .ok_or("that window is gone")?;
    w.capture_image().map_err(err)
}

pub fn focused_window_image() -> Result<RgbaImage, String> {
    let me = std::process::id();
    let w = xcap::Window::all()
        .map_err(err)?
        .into_iter()
        .find(|w| w.is_focused().unwrap_or(false) && w.pid().ok() != Some(me))
        .ok_or("no focused window")?;
    w.capture_image().map_err(err)
}

#[cfg(target_os = "linux")]
mod cursor {
    use image::{Rgba, RgbaImage};
    use x11rb::protocol::xfixes::ConnectionExt as _;

    pub struct Cursor {
        x: i32,
        y: i32,
        img: RgbaImage,
    }

    /// The pointer image and its top-left in desktop pixels (XFixes).
    pub fn grab() -> Option<Cursor> {
        let (conn, _) = x11rb::connect(None).ok()?;
        conn.xfixes_query_version(5, 0).ok()?.reply().ok()?;
        let r = conn.xfixes_get_cursor_image().ok()?.reply().ok()?;
        let mut img = RgbaImage::new(r.width.into(), r.height.into());
        for (px, argb) in img.pixels_mut().zip(r.cursor_image.iter()) {
            let [b, g, red, a] = argb.to_le_bytes();
            // XFixes gives premultiplied ARGB; image::overlay expects straight alpha.
            let un = |c: u8| {
                if a == 0 {
                    0
                } else {
                    ((c as u32 * 255) / a as u32).min(255) as u8
                }
            };
            *px = Rgba([un(red), un(g), un(b), a]);
        }
        Some(Cursor {
            x: i32::from(r.x) - i32::from(r.xhot),
            y: i32::from(r.y) - i32::from(r.yhot),
            img,
        })
    }

    pub fn composite(frame: &mut RgbaImage, c: &Cursor, fx: i32, fy: i32) {
        image::imageops::overlay(frame, &c.img, i64::from(c.x - fx), i64::from(c.y - fy));
    }
}

#[cfg(not(target_os = "linux"))]
mod cursor {
    // ponytail: pointer compositing is Linux-only until Plan 4.
    pub struct Cursor;
    pub fn grab() -> Option<Cursor> {
        None
    }
    pub fn composite(_frame: &mut image::RgbaImage, _c: &Cursor, _fx: i32, _fy: i32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: i32, y: i32, w: u32, h: u32) -> Frame {
        Frame {
            name: String::new(),
            x,
            y,
            image: RgbaImage::new(w, h),
        }
    }

    #[test]
    fn clamp_rect_rounds_and_clips() {
        assert_eq!(
            clamp_rect(10.4, 20.6, 100.0, 50.0, 1920, 1080),
            Some([10, 21, 100, 50])
        );
        assert_eq!(
            clamp_rect(-5.0, -5.0, 20.0, 20.0, 100, 100),
            Some([0, 0, 15, 15])
        );
        assert_eq!(
            clamp_rect(90.0, 90.0, 50.0, 50.0, 100, 100),
            Some([90, 90, 10, 10])
        );
        assert_eq!(clamp_rect(10.0, 10.0, 0.2, 30.0, 100, 100), None);
        assert_eq!(clamp_rect(200.0, 10.0, 10.0, 10.0, 100, 100), None);
    }

    #[test]
    fn frame_at_finds_the_monitor_under_a_point() {
        // The author's layout: DP-1 left, DP-4 middle (primary), HDMI-0 right.
        let fs = [
            frame(0, 224, 1920, 1080),
            frame(1920, 0, 2560, 1600),
            frame(4480, 252, 1920, 1080),
        ];
        assert_eq!(frame_at(&fs, 100, 500), 0);
        assert_eq!(frame_at(&fs, 3000, 10), 1);
        assert_eq!(frame_at(&fs, 5000, 1000), 2);
        assert_eq!(frame_at(&fs, 100, 10), 0);
    }

    #[test]
    fn crop_then_encode_gives_a_png_of_that_size() {
        let img = RgbaImage::from_pixel(50, 40, image::Rgba([255, 0, 0, 255]));
        let png = encode_png(&crop(&img, [5, 5, 20, 10])).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert_eq!(
            image::load_from_memory(&png)
                .unwrap()
                .to_rgba8()
                .dimensions(),
            (20, 10)
        );
    }
}
