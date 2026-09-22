# rshot Plan 1: Foundation + Linux Screenshots — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A daily-usable rshot on Ubuntu 24.04 / GNOME / X11. `Print`, `Shift+Print` and `Alt+Print` capture through rshot. The file lands in `~/Pictures/Screenshots`, and the clipboard holds the path, the file and the image. The Glass overlay, floating thumbnail, onboarding and settings all work, and a `.deb` installs it.

**Architecture:** One Tauri 2 process stays resident with a tray icon. It keeps a hidden, preloaded overlay window for each monitor and owns the clipboard. GNOME custom keybindings run `rshot capture …`, and the single-instance plugin forwards those calls to the resident process. Frozen monitor frames reach the overlay pages as raw RGBA through a binary IPC response. Pure logic (naming, geometry, GVariant edits, clipboard formats) lives in small functions with unit tests.

**Tech Stack:**
- Rust 1.96 (edition 2021) and Tauri 2.11. Plugins: single-instance, autostart, dialog, notification, opener, drag.
- Rust crates: xcap 0.9, clipboard-rs 0.3, image 0.25, x11rb 0.14 (xfixes), chrono, toml, dirs.
- Frontend: plain TypeScript and CSS built with Vite 7.3.6, tested with vitest 3.2.7. Fonts from `@fontsource/inter`.

**Spec:** `docs/superpowers/specs/2026-09-22-rshot-design.md`. The visual contract is in `docs/design/mockups/` (style **A · Glass**).

**Later plans (not in scope here):**
- Plan 2: image editor.
- Plan 3: recording and video trimming, which also takes over `Ctrl+Alt+Shift+R`.
- Plan 4: shortcut takeover, sounds and ffmpeg sidecars on Windows and macOS; `.rpm`, AppImage, NSIS and `.dmg` builds; `release.yml`; push.

## Global Constraints

- The only target tested at runtime is **Ubuntu 24.04, GNOME 46, X11**. The code must still **compile on Windows and macOS**, so every Linux-only item gets `#[cfg(target_os = "linux")]`: `x11rb`, `gsettings`, `pw-play` and the X11 clipboard formats.
- Capture file name: `Screenshot_YYYY-MM-DD_HH-MM-SS.png` (no spaces). A same-second collision adds `_2`, `_3` and so on. The folder is `<XDG Pictures>/Screenshots` unless configured otherwise.
- The clipboard gets the **absolute** path, never one starting with `~`.
- In "path + image" mode (the default), the clipboard holds `UTF8_STRING`, `text/plain;charset=utf-8`, `text/plain` (all three the path), plus `text/uri-list`, `x-special/gnome-copied-files` and `image/png`. "Path only" mode drops the last three.
- Files are always written atomically: write a temp file in the same directory, then rename it.
- Style tokens from the mockups:
  - accent `#0a84ff`, glass `rgba(40,40,46,.68)` + `blur(24px) saturate(180%)`, glass border `rgba(255,255,255,.12)`;
  - text `#f2f2f7`, OK `#30d158`, danger `#ff453a`, font Inter.
  - Match `docs/design/mockups/0{1,2,5}-*.png` for anything this plan doesn't spell out.
- Frontend is plain TS with no UI framework. The only npm dependencies are the ones listed in Task 1. Add no crate or npm package the plan doesn't name.
- Rust errors that cross IPC are `String`. Use the `crate::err` helper (`.map_err(err)`).
- Only touch the user's GNOME keybindings through rshot's own takeover/restore code, or through the Task 6 test binding, which must be removed in that same task.
- Commit at the end of every task. Every message ends with a blank line and then `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run `cargo` commands from `src-tauri/` and `npm` commands from the repo root.
- Some `cargo` steps (release builds, `generate_context!` in CI) need a built frontend, so run `npm run build` once first. Task 1 does this.

## Environment notes (the author's machine, read before any task)

- **Autonomous run.** The user is away. Don't wait for approval between tasks. Where a step says "ask the user", do the closest safe thing yourself and record what you did.
- **No `sudo`.**
  - Every build dependency is installed, except `xdotool` and the `libxcb-randr0-dev` symlink. The build links fine without the symlink.
  - Instead of `xdotool`, use the repo's `scripts/xdo.py` (XTest via ctypes):
    - `xdotool key X` → `python3 scripts/xdo.py key X`
    - `xdotool getactivewindow getwindowname` → `python3 scripts/xdo.py active`
    - `xdotool search --onlyvisible --name 'N' | wc -l` → `python3 scripts/xdo.py visible 'N'`
  - In Task 1 Step 1, check only `pkg-config` and `clang`.
  - In Task 13, don't `apt install` the `.deb`. Check it with `dpkg-deb -I`/`-c` and run `src-tauri/target/release/rshot` directly. Test the prerm logic by calling `rshot restore-shortcuts` yourself.
- **Leave the desktop as you found it.** After any test that takes over GNOME shortcuts, run `rshot restore-shortcuts` and confirm `gsettings get org.gnome.shell.keybindings show-screenshot-ui` returns `['Print']`. Restore `~/.config/rshot/config.toml` if a step moved it.
- **Self-sufficient `.deb`.** The package may depend only on libraries every stock Ubuntu desktop already has. Anything else gets bundled.

## Prerequisites (the user runs these once, before Task 1)

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev \
  libgbm-dev libdrm-dev libwayland-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev \
  xdotool
```

## File Map (end state of this plan)

```
package.json, package-lock.json, tsconfig.json, vite.config.ts, README.md
.github/workflows/ci.yml
docs/perf.md                              # Task 6 measurements
src/shared/glass.css                      # tokens + shared controls (Glass)
src/shared/icons.ts                       # SVG sprite + mountIcons()
src/shared/ipc.ts                         # typed invoke() wrappers, shared types
src/overlay/{index.html,overlay.css,main.ts,selection.ts,selection.test.ts,options.ts}
src/countdown/{index.html,main.ts}
src/thumbnail/{index.html,thumbnail.css,main.ts}
src/settings/{index.html,settings.css,main.ts,keys.ts,keys.test.ts}
src/onboarding/{index.html,main.ts}
src-tauri/Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json
src-tauri/icons/*                         # generated
src-tauri/sounds/shutter.wav              # generated
src-tauri/deb/prerm.sh
src-tauri/src/main.rs                     # AppState, builder, dispatch, err()
src-tauri/src/cli.rs                      # argv → Cmd
src-tauri/src/store.rs                    # Config (TOML), folders, naming, atomic write
src-tauri/src/clipboard.rs                # clipboard thread + per-OS payload
src-tauri/src/capture.rs                  # frames, crop, PNG, windows, cursor
src-tauri/src/pipeline.rs                 # finish_capture, immediate captures, notify, shutter
src-tauri/src/ui.rs                       # tray, overlay windows, popups, settings/onboarding windows
src-tauri/src/overlay.rs                  # session, frame IPC, targets, timer countdown
src-tauri/src/thumbnail.rs                # thumbnail commands + path guard
src-tauri/src/settings.rs                 # settings/onboarding commands
src-tauri/src/shortcuts/{mod.rs,gvariant.rs,gnome.rs}
```

---

### Task 1: Scaffold the Tauri app, CLI, tray, single instance

**Files:**
- Create: `package.json` (via npm), `tsconfig.json`, `vite.config.ts`, `src/placeholder/index.html`
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/icons/*`
- Create: `src-tauri/src/main.rs`, `src-tauri/src/cli.rs`, `src-tauri/src/ui.rs`
- Test: unit tests inside `src-tauri/src/cli.rs`

**Interfaces:**
- Produces: `cli::Cmd { Daemon, CaptureArea, CaptureScreen, CaptureWindow, RestoreShortcuts }`, `cli::parse(&[String]) -> Result<Cmd, String>`, `cli::USAGE`, `crate::dispatch(&AppHandle, Cmd)`, `crate::err(impl Display) -> String`, `ui::create_tray(&AppHandle) -> tauri::Result<()>`.

- [ ] **Step 1: Check the prerequisites are installed**

Run: `pkg-config --exists webkit2gtk-4.1 ayatana-appindicator3-0.1 libpipewire-0.3 gbm && which xdotool clang && echo OK`
Expected: `OK` followed by two paths. If this fails, stop and ask the user to run the Prerequisites command.

- [ ] **Step 2: Create the npm project**

```bash
npm init -y >/dev/null
npm pkg set name=rshot version=0.1.0 private=true type=module
npm pkg set scripts.dev="vite" scripts.build="tsc --noEmit && vite build" scripts.test="vitest run" scripts.tauri="tauri"
npm pkg delete main description keywords author license
npm install -E @tauri-apps/api@2.11.1 @crabnebula/tauri-plugin-drag@2.1.0 @fontsource/inter@5.3.0
npm install -E -D @tauri-apps/cli@2.11.5 typescript@5.9.3 vite@7.3.6 vitest@3.2.7
```

- [ ] **Step 3: Write `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "types": ["vite/client"],
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true,
    "isolatedModules": true
  },
  "include": ["src"]
}
```

- [ ] **Step 4: Write `vite.config.ts`**

This is a multi-page build: every page folder in `src/` becomes `dist/<page>/index.html`. Later tasks add their page to `pages`.

```ts
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

const pages = ['placeholder'];

export default defineConfig({
  root: 'src',
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: Object.fromEntries(pages.map((p) => [p, fileURLToPath(new URL(`./src/${p}/index.html`, import.meta.url))])),
    },
  },
  test: { include: ['**/*.test.ts'] },
});
```

- [ ] **Step 5: Add a placeholder page so the first build has an input** (Task 5 removes it)

`src/placeholder/index.html`:

```html
<!doctype html>
<html lang="en"><head><meta charset="utf-8" /><title>rshot</title></head><body></body></html>
```

Run: `npm run build`
Expected: finishes without errors and writes `dist/placeholder/index.html`.

- [ ] **Step 6: Write `src-tauri/Cargo.toml` and `src-tauri/build.rs`**

```toml
[package]
name = "rshot"
version = "0.1.0"
description = "Screenshots whose path lands on your clipboard"
edition = "2021"
license = "MIT"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2.11", features = ["tray-icon", "macos-private-api"] }
tauri-plugin-single-instance = "2.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
// src-tauri/build.rs
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 7: Write `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "rshot",
  "version": "0.1.0",
  "identifier": "io.github.kecsou.rshot",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [],
    "macOSPrivateApi": true,
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["deb"],
    "category": "Utility",
    "shortDescription": "Screenshots whose path lands on your clipboard",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
}
```

- [ ] **Step 8: Write `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Every rshot window",
  "windows": ["*"],
  "permissions": ["core:default", "core:window:allow-start-dragging"]
}
```

- [ ] **Step 9: Generate the app icon**

The design is the Glass logo from mockup 05: a blue-to-indigo rounded square with white corner brackets and a ring.

```bash
mkdir -p src-tauri/icons
python3 - <<'EOF'
from PIL import Image, ImageDraw
S = 1024
grad = Image.new('RGBA', (S, S))
g = ImageDraw.Draw(grad)
for y in range(S):
    t = y / (S - 1)
    g.line([(0, y), (S, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip((10, 132, 255), (94, 92, 230))) + (255,))
mask = Image.new('L', (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([64, 64, S - 64, S - 64], radius=200, fill=255)
img = Image.new('RGBA', (S, S), (0, 0, 0, 0))
img.paste(grad, (0, 0), mask)
d = ImageDraw.Draw(img)
w, m, L = 64, 250, 150
for x, y, dx, dy in [(m, m, 1, 1), (S - m, m, -1, 1), (m, S - m, 1, -1), (S - m, S - m, -1, -1)]:
    d.line([(x, y), (x + dx * L, y)], fill='white', width=w)
    d.line([(x, y), (x, y + dy * L)], fill='white', width=w)
d.ellipse([S / 2 - 95, S / 2 - 95, S / 2 + 95, S / 2 + 95], outline='white', width=w)
img.save('src-tauri/icons/app-icon.png')
EOF
npx tauri icon src-tauri/icons/app-icon.png
rm -rf src-tauri/icons/android src-tauri/icons/ios
ls src-tauri/icons
```

Expected: the listing includes `32x32.png 128x128.png 128x128@2x.png icon.icns icon.ico app-icon.png`.

- [ ] **Step 10: Write the failing CLI test**

`src-tauri/src/cli.rs`:

```rust
//! Command line: `rshot`, `rshot capture area|screen|window`, `rshot restore-shortcuts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Daemon,
    CaptureArea,
    CaptureScreen,
    CaptureWindow,
    RestoreShortcuts,
}

pub const USAGE: &str = "usage: rshot [capture area|screen|window] [restore-shortcuts]";

/// Parses the arguments that follow the program name.
pub fn parse(_args: &[String]) -> Result<Cmd, String> {
    Err(USAGE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Cmd, String> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(p(&[]), Ok(Cmd::Daemon));
        assert_eq!(p(&["capture", "area"]), Ok(Cmd::CaptureArea));
        assert_eq!(p(&["capture", "screen"]), Ok(Cmd::CaptureScreen));
        assert_eq!(p(&["capture", "window"]), Ok(Cmd::CaptureWindow));
        assert_eq!(p(&["restore-shortcuts"]), Ok(Cmd::RestoreShortcuts));
    }

    #[test]
    fn rejects_unknown_input() {
        assert_eq!(p(&["capture"]), Err(USAGE.to_string()));
        assert_eq!(p(&["capture", "moon"]), Err(USAGE.to_string()));
        assert_eq!(p(&["--help"]), Err(USAGE.to_string()));
    }
}
```

`src-tauri/src/ui.rs`:

```rust
//! Windows and tray.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit = MenuItem::with_id(app, "quit", "Quit rshot", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundle icon is configured"))
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            if e.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;
    Ok(())
}
```

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod ui;

use tauri::{AppHandle, RunEvent};

/// Error adapter for IPC: every command error is a `String`.
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = cli::parse(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });

    let app = tauri::Builder::default()
        // Must stay the first plugin: a second `rshot …` forwards its argv here and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            match cli::parse(argv.get(1..).unwrap_or_default()) {
                Ok(cmd) => dispatch(app, cmd),
                Err(e) => eprintln!("rshot: {e}"),
            }
        }))
        .setup(move |app| {
            ui::create_tray(app.handle())?;
            dispatch(app.handle(), cmd);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build rshot");

    app.run(|_app, event| {
        // Live in the tray: closing the last window must not quit; only app.exit(code) does.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}

pub fn dispatch(_app: &AppHandle, cmd: cli::Cmd) {
    eprintln!("rshot: dispatch {cmd:?}");
}
```

- [ ] **Step 11: Run the test and confirm it fails**

Run: `cd src-tauri && cargo test cli`
Expected: it compiles (the first build takes several minutes). `parses_every_command` FAILS: `left: Err("usage: …")`, `right: Ok(Daemon)`.

- [ ] **Step 12: Implement `parse`**

Replace the body of `parse` in `cli.rs`:

```rust
pub fn parse(args: &[String]) -> Result<Cmd, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => Ok(Cmd::Daemon),
        ["capture", "area"] => Ok(Cmd::CaptureArea),
        ["capture", "screen"] => Ok(Cmd::CaptureScreen),
        ["capture", "window"] => Ok(Cmd::CaptureWindow),
        ["restore-shortcuts"] => Ok(Cmd::RestoreShortcuts),
        _ => Err(USAGE.to_string()),
    }
}
```

Run: `cargo test cli`
Expected: `2 passed`.

- [ ] **Step 13: Run the app and check the tray and single-instance forwarding**

```bash
npm run tauri dev > /tmp/rshot-dev.log 2>&1 &   # from the repo root; wait until the log shows "rshot: dispatch Daemon"
./src-tauri/target/debug/rshot capture area; echo "exit=$?"
grep "dispatch" /tmp/rshot-dev.log
```

Expected:
- The rshot icon appears in the GNOME top bar, and its menu shows "Quit rshot".
- The second command prints `exit=0` at once.
- The log contains `rshot: dispatch Daemon` and then `rshot: dispatch CaptureArea`.
- Choosing **Quit rshot** in the tray ends the dev process.

- [ ] **Step 14: Commit**

```bash
git add package.json package-lock.json tsconfig.json vite.config.ts src src-tauri
git commit -m "feat: scaffold Tauri app with tray, CLI and single instance

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(`src-tauri/target` and `src-tauri/gen` must not be committed. Check with `git status`. If they show up, add `src-tauri/target/` and `src-tauri/gen/` to `.gitignore` and amend the commit.)

---

### Task 2: Config, folders, naming, atomic writes (`store.rs`)

**Files:**
- Create: `src-tauri/src/store.rs`
- Modify: `src-tauri/Cargo.toml` (deps), `src-tauri/src/main.rs` (mod + state)
- Test: unit tests inside `store.rs`

**Interfaces:**
- Produces:
  - `store::Config` (serde, `#[serde(default)]`) with the fields `screenshots_dir: Option<PathBuf>`, `clipboard_mode: ClipboardMode`, `show_thumbnail: bool`, `shutter_sound: bool`, `remember_selection: bool`, `show_pointer: bool`, `timer_secs: u8`, `last_selection: Option<Selection>`, `hints_shown: u32`, `onboarded: bool`, `takeover: bool`, `shortcuts: Shortcuts`, `gnome_backup: Option<BTreeMap<String, String>>`.
  - `ClipboardMode { PathAndImage, PathOnly }`, serialised as `"path-and-image"` / `"path-only"`.
  - `Shortcuts { area, screen, window: String }`, defaulting to `"Print"`, `"Shift+Print"`, `"Alt+Print"`.
  - `Selection { monitor: String, x, y, w, h: u32 }`.
  - Functions:

    ```rust
    config_path() -> PathBuf
    load_config() -> Config
    load_config_from(&Path) -> Config
    save_config(&Config) -> io::Result<()>
    save_config_to(&Path, &Config) -> io::Result<()>
    screenshots_dir(&Config) -> PathBuf
    dir_setting(&str) -> Option<PathBuf>
    capture_stem(prefix: &str, NaiveDateTime) -> String
    unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf
    new_screenshot_path(&Config) -> io::Result<PathBuf>
    write_atomic(&Path, &[u8]) -> io::Result<()>
    ```

  - `AppState { config: Mutex<Config> }` in `main.rs`.

- [ ] **Step 1: Add dependencies**

```bash
cd src-tauri && cargo add toml@1 dirs@7 && cargo add chrono@0.4 --no-default-features --features clock
```

- [ ] **Step 2: Write `store.rs` with stub bodies and the failing tests**

```rust
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

pub fn load_config_from(_p: &Path) -> Config {
    Config::default()
}

pub fn save_config(c: &Config) -> io::Result<()> {
    save_config_to(&config_path(), c)
}

pub fn save_config_to(_p: &Path, _c: &Config) -> io::Result<()> {
    Ok(())
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

pub fn capture_stem(_prefix: &str, _t: NaiveDateTime) -> String {
    String::new()
}

pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    dir.join(format!("{stem}.{ext}"))
}

pub fn new_screenshot_path(c: &Config) -> io::Result<PathBuf> {
    let dir = screenshots_dir(c);
    fs::create_dir_all(&dir)?;
    Ok(unique_path(&dir, &capture_stem("Screenshot", Local::now().naive_local()), "png"))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)
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
```

In `main.rs`, add `mod store;` next to the other `mod` lines.

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test store`
Expected: `stem_has_no_spaces`, `unique_path_appends_a_counter`, `write_atomic_creates_dirs_and_leaves_no_temp_file` and `config_round_trips_and_fills_defaults` FAIL. `dir_setting_maps_the_default_folder_to_none` passes.

- [ ] **Step 4: Implement the stubs**

Replace these functions in `store.rs`:

```rust
pub fn load_config_from(p: &Path) -> Config {
    match fs::read_to_string(p) {
        Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
            eprintln!("rshot: ignoring invalid {}: {e}", p.display());
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

pub fn save_config_to(p: &Path, c: &Config) -> io::Result<()> {
    let s = toml::to_string(c).map_err(io::Error::other)?;
    write_atomic(p, s.as_bytes())
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
```

Run: `cargo test store`
Expected: `5 passed`.

- [ ] **Step 5: Keep the config in app state**

In `main.rs`, add this after the `err` function:

```rust
pub struct AppState {
    pub config: std::sync::Mutex<store::Config>,
}

impl AppState {
    fn new() -> Self {
        Self { config: std::sync::Mutex::new(store::load_config()) }
    }
}
```

In the builder, add `.manage(AppState::new())` directly after the single-instance `.plugin(…)` call.

Run: `cargo build`
Expected: builds. (Warnings about unused functions are expected until later tasks use them.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri && git commit -m "feat: config, capture naming and atomic writes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Clipboard owner thread and per-OS payload (`clipboard.rs`)

**Files:**
- Create: `src-tauri/src/clipboard.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`
- Test: unit tests inside `clipboard.rs` (Linux only)

**Interfaces:**
- Consumes: `store::ClipboardMode`.
- Produces:
  - `clipboard::Clipboard` (Send + Sync) with `spawn() -> Clipboard`, `copy_capture(&self, path: &Path, png: Option<&[u8]>, mode: ClipboardMode) -> Result<(), String>` and `copy_image(&self, png: &[u8]) -> Result<(), String>`.
  - Linux only: `clipboard::file_uri(&Path) -> String` and `clipboard::linux_formats(&Path, Option<&[u8]>) -> Vec<(&'static str, Vec<u8>)>`.
  - `AppState.clipboard`.

- [ ] **Step 1: Add the dependency**

Run: `cargo add clipboard-rs@0.3`

- [ ] **Step 2: Write `clipboard.rs` with stub helpers and the failing tests**

```rust
//! Clipboard: one thread owns the OS clipboard for the life of the daemon (on X11, pastes are
//! served by the owning process, so the owner must outlive the capture).

use crate::store::ClipboardMode;
use clipboard_rs::{Clipboard as _, ClipboardContent, ClipboardContext};
use std::{path::Path, sync::mpsc, thread};

type Job = (Vec<ClipboardContent>, mpsc::Sender<Result<(), String>>);

pub struct Clipboard {
    tx: mpsc::Sender<Job>,
}

impl Clipboard {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        thread::spawn(move || {
            let ctx = ClipboardContext::new();
            for (contents, reply) in rx {
                let result = match &ctx {
                    Ok(c) => c.set(contents).map_err(|e| e.to_string()),
                    Err(e) => Err(format!("clipboard unavailable: {e}")),
                };
                let _ = reply.send(result);
            }
        });
        Self { tx }
    }

    /// PathAndImage: path text + file (+ PNG when given). PathOnly: the path text alone (spec §2.4).
    pub fn copy_capture(&self, path: &Path, png: Option<&[u8]>, mode: ClipboardMode) -> Result<(), String> {
        match mode {
            ClipboardMode::PathAndImage => self.set(contents(path, png)),
            ClipboardMode::PathOnly => self.set(text_only(path)),
        }
    }

    /// Image only — used when saving failed, so the capture isn't lost.
    pub fn copy_image(&self, png: &[u8]) -> Result<(), String> {
        self.set(image_only(png))
    }

    fn set(&self, contents: Vec<ClipboardContent>) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        self.tx.send((contents, reply)).map_err(|e| e.to_string())?;
        answer.recv().map_err(|e| e.to_string())?
    }
}

/// RFC 8089 file URI with percent-encoding (folders may be localised, e.g. "Vidéos").
#[cfg(target_os = "linux")]
pub fn file_uri(_path: &Path) -> String {
    String::new()
}

/// X11 targets rshot serves, in order.
#[cfg(target_os = "linux")]
pub fn linux_formats(_path: &Path, _png: Option<&[u8]>) -> Vec<(&'static str, Vec<u8>)> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn contents(path: &Path, png: Option<&[u8]>) -> Vec<ClipboardContent> {
    linux_formats(path, png)
        .into_iter()
        .map(|(format, bytes)| match format {
            "UTF8_STRING" => ClipboardContent::Text(String::from_utf8_lossy(&bytes).into_owned()),
            _ => ClipboardContent::Other(format.to_string(), bytes),
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn image_only(png: &[u8]) -> Vec<ClipboardContent> {
    vec![ClipboardContent::Other("image/png".into(), png.to_vec())]
}

/// Path as text only: the three text targets, no file or image targets.
#[cfg(target_os = "linux")]
fn text_only(path: &Path) -> Vec<ClipboardContent> {
    contents(path, None).into_iter().take(3).collect()
}

#[cfg(not(target_os = "linux"))]
fn contents(path: &Path, png: Option<&[u8]>) -> Vec<ClipboardContent> {
    let p = path.to_string_lossy().into_owned();
    let mut v = vec![ClipboardContent::Text(p.clone()), ClipboardContent::Files(vec![p])];
    v.extend(image_only(png.unwrap_or_default()));
    v
}

#[cfg(not(target_os = "linux"))]
fn text_only(path: &Path) -> Vec<ClipboardContent> {
    vec![ClipboardContent::Text(path.to_string_lossy().into_owned())]
}

#[cfg(not(target_os = "linux"))]
fn image_only(png: &[u8]) -> Vec<ClipboardContent> {
    use clipboard_rs::{common::RustImage, RustImageData};
    RustImageData::from_bytes(png).map(ClipboardContent::Image).into_iter().collect()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn file_uri_percent_encodes() {
        assert_eq!(file_uri(Path::new("/home/k/Vidéos/a b.png")), "file:///home/k/Vid%C3%A9os/a%20b.png");
        assert_eq!(
            file_uri(Path::new("/tmp/Screenshot_2026-09-22_20-41-07.png")),
            "file:///tmp/Screenshot_2026-09-22_20-41-07.png"
        );
    }

    #[test]
    fn formats_carry_path_uri_and_optional_png() {
        let p = Path::new("/tmp/s.png");
        let f = linux_formats(p, Some(b"PNG"));
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, b)| b.clone()).unwrap();
        assert_eq!(get("UTF8_STRING"), b"/tmp/s.png");
        assert_eq!(get("text/plain;charset=utf-8"), b"/tmp/s.png");
        assert_eq!(get("text/plain"), b"/tmp/s.png");
        assert_eq!(get("text/uri-list"), b"file:///tmp/s.png\r\n");
        assert_eq!(get("x-special/gnome-copied-files"), b"copy\nfile:///tmp/s.png");
        assert_eq!(get("image/png"), b"PNG");
        assert!(linux_formats(p, None).iter().all(|(n, _)| *n != "image/png"));
    }

    #[test]
    fn text_only_serves_just_the_path_text() {
        let formats: Vec<String> = text_only(Path::new("/tmp/s.png"))
            .into_iter()
            .map(|c| match c {
                ClipboardContent::Text(_) => "UTF8_STRING".to_string(),
                ClipboardContent::Other(f, _) => f,
                _ => "unexpected".to_string(),
            })
            .collect();
        assert_eq!(formats, ["UTF8_STRING", "text/plain;charset=utf-8", "text/plain"]);
    }
}
```

In `main.rs`, add `mod clipboard;`. Change `AppState` and `AppState::new` to:

```rust
pub struct AppState {
    pub config: std::sync::Mutex<store::Config>,
    pub clipboard: clipboard::Clipboard,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: std::sync::Mutex::new(store::load_config()),
            clipboard: clipboard::Clipboard::spawn(),
        }
    }
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test clipboard`
Expected: all three tests FAIL (empty URI / `unwrap()` on `None` / empty format list).

- [ ] **Step 4: Implement the helpers**

```rust
#[cfg(target_os = "linux")]
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut s = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

#[cfg(target_os = "linux")]
pub fn linux_formats(path: &Path, png: Option<&[u8]>) -> Vec<(&'static str, Vec<u8>)> {
    let text = path.to_string_lossy().into_owned().into_bytes();
    let uri = file_uri(path);
    let mut v = vec![
        ("UTF8_STRING", text.clone()),
        ("text/plain;charset=utf-8", text.clone()),
        ("text/plain", text),
        ("text/uri-list", format!("{uri}\r\n").into_bytes()),
        ("x-special/gnome-copied-files", format!("copy\n{uri}").into_bytes()),
    ];
    if let Some(p) = png {
        v.push(("image/png", p.to_vec()));
    }
    v
}
```

Run: `cargo test clipboard`
Expected: `3 passed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri && git commit -m "feat: clipboard owner thread with path, file and image formats

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Capture + pipeline — `rshot capture screen|window` saves, copies, plays the shutter

**Files:**
- Create: `src-tauri/src/capture.rs`, `src-tauri/src/pipeline.rs`, `src-tauri/sounds/shutter.wav`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (resources), `src-tauri/src/main.rs`, `src-tauri/src/ui.rs` (tray items)
- Test: unit tests inside `capture.rs`

**Interfaces:**
- Consumes: `store::{new_screenshot_path, write_atomic, screenshots_dir}`, `Clipboard::{copy_capture, copy_image}`.
- Produces:
  - `capture::Frame { name: String, x: i32, y: i32, image: RgbaImage }` and `capture::WinRect { id: u32, title, app: String, x: i32, y: i32, w: u32, h: u32 }` (Serialize).
  - `capture` functions:

    ```rust
    grab_all(show_pointer: bool) -> Result<Vec<Frame>, String>
    frame_at(&[Frame], i32, i32) -> usize
    clamp_rect(x, y, w, h: f64, fw, fh: u32) -> Option<[u32; 4]>
    crop(&RgbaImage, [u32; 4]) -> RgbaImage
    encode_png(&RgbaImage) -> Result<Vec<u8>, String>
    windows_on(&Frame) -> Vec<WinRect>
    window_image(id: u32) -> Result<RgbaImage, String>
    focused_window_image() -> Result<RgbaImage, String>
    ```

  - `pipeline::finish_capture(&AppHandle, RgbaImage) -> Result<PathBuf, String>`, `pipeline::capture_screen_now(&AppHandle) -> Result<(), String>`, `pipeline::capture_window_now(&AppHandle) -> Result<(), String>`, `pipeline::notify(&AppHandle, &str)`.
  - `AppState.last_capture: Mutex<Option<PathBuf>>`.

- [ ] **Step 1: Add dependencies**

```bash
cargo add xcap@0.9 tauri-plugin-notification@2.4
cargo add image@0.25 --no-default-features --features png
cargo add x11rb@0.14 --features xfixes --target 'cfg(target_os = "linux")'
```

- [ ] **Step 2: Write `capture.rs` with stubs and the failing tests**

```rust
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

pub fn grab_all(show_pointer: bool) -> Result<Vec<Frame>, String> {
    let cursor = if show_pointer { cursor::grab() } else { None };
    xcap::Monitor::all()
        .map_err(err)?
        .iter()
        .map(|m| {
            let mut image = m.capture_image().map_err(err)?;
            let (x, y) = (m.x().map_err(err)?, m.y().map_err(err)?);
            if let Some(c) = &cursor {
                cursor::composite(&mut image, c, x, y);
            }
            Ok(Frame { name: m.name().unwrap_or_default(), x, y, image })
        })
        .collect()
}

/// Index of the frame containing the desktop point, or 0.
pub fn frame_at(_frames: &[Frame], _px: i32, _py: i32) -> usize {
    usize::MAX
}

/// Rounds and clips an image-pixel rect to the frame; `None` when nothing is left.
pub fn clamp_rect(_x: f64, _y: f64, _w: f64, _h: f64, _fw: u32, _fh: u32) -> Option<[u32; 4]> {
    None
}

pub fn crop(img: &RgbaImage, r: [u32; 4]) -> RgbaImage {
    image::imageops::crop_imm(img, r[0], r[1], r[2], r[3]).to_image()
}

pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgba8)
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
            let hits = r.w > 0 && r.h > 0 && r.x < fw && r.y < fh && r.x + r.w as i32 > 0 && r.y + r.h as i32 > 0;
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
            let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255) / a as u32).min(255) as u8 };
            *px = Rgba([un(red), un(g), un(b), a]);
        }
        Some(Cursor { x: i32::from(r.x) - i32::from(r.xhot), y: i32::from(r.y) - i32::from(r.yhot), img })
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
        Frame { name: String::new(), x, y, image: RgbaImage::new(w, h) }
    }

    #[test]
    fn clamp_rect_rounds_and_clips() {
        assert_eq!(clamp_rect(10.4, 20.6, 100.0, 50.0, 1920, 1080), Some([10, 21, 100, 50]));
        assert_eq!(clamp_rect(-5.0, -5.0, 20.0, 20.0, 100, 100), Some([0, 0, 15, 15]));
        assert_eq!(clamp_rect(90.0, 90.0, 50.0, 50.0, 100, 100), Some([90, 90, 10, 10]));
        assert_eq!(clamp_rect(10.0, 10.0, 0.2, 30.0, 100, 100), None);
        assert_eq!(clamp_rect(200.0, 10.0, 10.0, 10.0, 100, 100), None);
    }

    #[test]
    fn frame_at_finds_the_monitor_under_a_point() {
        // The author's layout: DP-1 left, DP-4 middle (primary), HDMI-0 right.
        let fs = [frame(0, 224, 1920, 1080), frame(1920, 0, 2560, 1600), frame(4480, 252, 1920, 1080)];
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
        assert_eq!(image::load_from_memory(&png).unwrap().to_rgba8().dimensions(), (20, 10));
    }
}
```

In `main.rs`, add `mod capture;`.

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test capture`
Expected: `clamp_rect_rounds_and_clips` and `frame_at_finds_the_monitor_under_a_point` FAIL. `crop_then_encode_gives_a_png_of_that_size` passes.

- [ ] **Step 4: Implement `frame_at` and `clamp_rect`**

```rust
pub fn frame_at(frames: &[Frame], px: i32, py: i32) -> usize {
    frames
        .iter()
        .position(|f| {
            px >= f.x && py >= f.y && px < f.x + f.image.width() as i32 && py < f.y + f.image.height() as i32
        })
        .unwrap_or(0)
}

pub fn clamp_rect(x: f64, y: f64, w: f64, h: f64, fw: u32, fh: u32) -> Option<[u32; 4]> {
    let (fw, fh) = (fw as f64, fh as f64);
    let x0 = x.round().clamp(0.0, fw);
    let y0 = y.round().clamp(0.0, fh);
    let x1 = (x + w).round().clamp(0.0, fw);
    let y1 = (y + h).round().clamp(0.0, fh);
    (x1 > x0 && y1 > y0).then(|| [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}
```

Run: `cargo test capture`
Expected: `3 passed`.

- [ ] **Step 5: Generate the shutter sound and bundle it**

```bash
mkdir -p src-tauri/sounds
python3 - <<'EOF'
import math, random, struct, wave
random.seed(7)
rate, n = 44100, int(44100 * 0.12)
out = bytearray()
for i in range(n):
    t = i / rate
    env = math.exp(-t * 60) + (0.6 * math.exp(-(t - 0.045) * 70) if t > 0.045 else 0)
    s = (random.uniform(-1, 1) * 0.6 + math.sin(2 * math.pi * 2200 * t) * 0.4) * env * 0.5
    out += struct.pack('<h', int(max(-1.0, min(1.0, s)) * 32767))
with wave.open('src-tauri/sounds/shutter.wav', 'wb') as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(rate); w.writeframes(bytes(out))
EOF
pw-play src-tauri/sounds/shutter.wav   # expect a short camera click
```

In `tauri.conf.json`, add `"resources": { "sounds/shutter.wav": "sounds/shutter.wav" }` inside `"bundle"`.

- [ ] **Step 6: Write `pipeline.rs`**

```rust
//! What happens after pixels are chosen: PNG, atomic save, clipboard, sound, feedback.

use crate::{capture, err, store, AppState};
use image::RgbaImage;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn notify(app: &AppHandle, msg: &str) {
    eprintln!("rshot: {msg}");
    let _ = app.notification().builder().title("rshot").body(msg).show();
}

/// Saves `img`, puts it on the clipboard and gives feedback. Returns the saved path.
pub fn finish_capture(app: &AppHandle, img: RgbaImage) -> Result<PathBuf, String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();
    let png = capture::encode_png(&img)?;
    let saved = store::new_screenshot_path(&cfg).and_then(|p| store::write_atomic(&p, &png).map(|()| p));
    let path = match saved {
        Ok(p) => p,
        Err(e) => {
            // Never lose the capture: fall back to the image alone.
            let copied = state.clipboard.copy_image(&png).is_ok();
            return Err(format!(
                "Couldn't save to {}: {e}{}",
                store::screenshots_dir(&cfg).display(),
                if copied { " (image copied)" } else { "" }
            ));
        }
    };
    let copied = state.clipboard.copy_capture(&path, Some(&png), cfg.clipboard_mode);
    *state.last_capture.lock().unwrap() = Some(path.clone());
    if cfg.shutter_sound {
        play_shutter(app);
    }
    if let Err(e) = copied {
        notify(app, &format!("Saved {}, but copying failed: {e}", path.display()));
    }
    Ok(path)
}

pub fn capture_screen_now(app: &AppHandle) -> Result<(), String> {
    let show_pointer = app.state::<AppState>().config.lock().unwrap().show_pointer;
    let mut frames = capture::grab_all(show_pointer)?;
    let pos = app.cursor_position().map_err(err)?;
    let i = capture::frame_at(&frames, pos.x as i32, pos.y as i32);
    finish_capture(app, frames.swap_remove(i).image).map(|_| ())
}

pub fn capture_window_now(app: &AppHandle) -> Result<(), String> {
    finish_capture(app, capture::focused_window_image()?).map(|_| ())
}

#[cfg(target_os = "linux")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    if let Ok(wav) = app.path().resolve("sounds/shutter.wav", BaseDirectory::Resource) {
        // A thread waits on the player so no zombie process is left behind.
        std::thread::spawn(move || {
            if std::process::Command::new("pw-play").arg(&wav).status().is_err() {
                let _ = std::process::Command::new("paplay").arg(&wav).status();
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn play_shutter(_app: &AppHandle) {} // ponytail: Windows/macOS sound arrives in Plan 4
```

- [ ] **Step 7: Wire `main.rs`**

- Add `mod pipeline;`.
- Add `pub last_capture: std::sync::Mutex<Option<std::path::PathBuf>>,` to `AppState`, and `last_capture: std::sync::Mutex::new(None),` to `new()`.
- Add `.plugin(tauri_plugin_notification::init())` after `.manage(…)`.
- Replace `dispatch`:

```rust
pub fn dispatch(app: &AppHandle, cmd: cli::Cmd) {
    use cli::Cmd::*;
    let result = match cmd {
        Daemon | RestoreShortcuts => Ok(()),
        CaptureArea => pipeline::capture_screen_now(app), // Task 5 switches this to the overlay
        CaptureScreen => pipeline::capture_screen_now(app),
        CaptureWindow => pipeline::capture_window_now(app),
    };
    if let Err(e) = result {
        pipeline::notify(app, &e);
    }
}
```

Replace `create_tray` in `ui.rs`. Captures started from the menu wait 300 ms, so the closing menu isn't in the shot:

```rust
use crate::cli::Cmd;
use std::time::Duration;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &item("area", "Capture Area")?,
            &item("screen", "Capture Screen")?,
            &item("window", "Capture Window")?,
            &sep,
            &item("quit", "Quit rshot")?,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundle icon is configured"))
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            let cmd = match e.id.as_ref() {
                "quit" => return app.exit(0),
                "area" => Cmd::CaptureArea,
                "screen" => Cmd::CaptureScreen,
                "window" => Cmd::CaptureWindow,
                _ => return,
            };
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                crate::dispatch(&app, cmd);
            });
        })
        .build(app)?;
    Ok(())
}
```

- [ ] **Step 8: Verify end to end**

```bash
npm run tauri dev > /tmp/rshot-dev.log 2>&1 &    # wait for "dispatch Daemon"
./src-tauri/target/debug/rshot capture screen
ls -t ~/Pictures/Screenshots | head -1
xclip -selection clipboard -o; echo
xclip -selection clipboard -t TARGETS -o
xclip -selection clipboard -t image/png -o | file -
```

Expected:
- The newest file is `Screenshot_2026-…_…-…-….png`.
- The clipboard text is its **absolute** path.
- TARGETS lists `UTF8_STRING`, `text/uri-list`, `x-special/gnome-copied-files` and `image/png`.
- `file` reports `PNG image data, W x H` with the size of the monitor under the pointer.
- You hear the shutter.

Then run `./src-tauri/target/debug/rshot capture window` with a terminal focused. A PNG of the terminal window alone is saved.

- [ ] **Step 9: Commit**

```bash
git add src-tauri && git commit -m "feat: capture pipeline with immediate screen and window captures

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Overlay plumbing — preloaded windows, frame IPC, capture targets

**Files:**
- Create: `src-tauri/src/overlay.rs`, `src/shared/glass.css`, `src/shared/ipc.ts`, `src/overlay/index.html`, `src/overlay/overlay.css`, `src/overlay/main.ts` (minimal; Task 7 replaces it)
- Delete: `src/placeholder/`
- Modify: `vite.config.ts`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `capture::{grab_all, frame_at, windows_on, window_image, clamp_rect, crop}`, `pipeline::{finish_capture, notify}`.
- Produces:
  - Rust types: `overlay::Session { token, mode, frames, active, started }`, `overlay::Rect { x, y, w, h: f64 }`, `overlay::Target` (`{ kind: "area", rect } | { kind: "window", id, rect } | { kind: "screen" }`), `overlay::OverlayOptions` and `overlay::start(&AppHandle, mode: &str) -> Result<(), String>`.
  - Commands: `overlay_info`, `overlay_frame` (raw RGBA), `overlay_ready(token)`, `overlay_activate(token)`, `overlay_cancel`, `overlay_capture(token, target)`.
  - Events: `overlay:show` (token), `overlay:active` (index), `overlay:hide`.
  - `ui::{overlay_index, ensure_overlays, place_overlays, hide_overlays}`.
  - TS in `src/shared/ipc.ts`: the types `Rect`, `WinRect`, `OverlayOptions`, `OverlayInfo` and `Target`, and the functions `overlayInfo`, `overlayFrame`, `overlayReady`, `overlayActivate`, `overlayCancel` and `overlayCapture`.

- [ ] **Step 1: Switch Vite to the real pages**

```bash
rm -rf src/placeholder
```

In `vite.config.ts`, set `const pages = ['overlay'];`.

- [ ] **Step 2: Write `src/shared/glass.css`** (the tokens and controls every page shares; values come from the mockups)

```css
@import '@fontsource/inter/400.css';
@import '@fontsource/inter/500.css';
@import '@fontsource/inter/600.css';
@import '@fontsource/inter/700.css';

:root {
  --accent: #0a84ff;
  --ok: #30d158;
  --danger: #ff453a;
  --text: #f2f2f7;
  --text-2: rgba(255, 255, 255, 0.55);
  --text-3: rgba(255, 255, 255, 0.45);
  --glass: rgba(40, 40, 46, 0.68);
  --glass-strong: rgba(40, 40, 46, 0.78);
  --glass-border: rgba(255, 255, 255, 0.12);
  --surface: #1b1b1f;
  --surface-2: #232328;
  --line: rgba(255, 255, 255, 0.07);
  --fill: rgba(255, 255, 255, 0.08);
  --fill-on: rgba(255, 255, 255, 0.2);
  font-family: Inter, system-ui, sans-serif;
  color: var(--text);
  -webkit-font-smoothing: antialiased;
}

* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { background: transparent; user-select: none; -webkit-user-select: none; }
[hidden] { display: none !important; }
button { font: inherit; color: inherit; background: none; border: 0; cursor: default; }

.glass {
  background: var(--glass);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
  border: 1px solid var(--glass-border);
  box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5);
  color: var(--text);
}

.ic { width: 17px; height: 17px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; flex-shrink: 0; }
.ic.sm { width: 13px; height: 13px; }

.btn { width: 36px; height: 30px; display: grid; place-items: center; border-radius: 8px; color: #d9d9e0; }
.btn:hover { background: rgba(255, 255, 255, 0.1); }
.btn.on { background: rgba(255, 255, 255, 0.17); color: #fff; }
.b1 { height: 30px; padding: 0 16px; border-radius: 8px; display: flex; align-items: center; font: 600 12px Inter; background: var(--accent); color: #fff; }
.b2 { height: 30px; padding: 0 12px; border-radius: 8px; display: flex; align-items: center; gap: 6px; font: 600 12px Inter; background: rgba(255, 255, 255, 0.1); color: #fff; }

.seg { display: flex; padding: 2px; border-radius: 8px; background: var(--fill); }
.seg button { flex: 1; padding: 4px 10px; border-radius: 6px; color: rgba(255, 255, 255, 0.75); font: 500 12px Inter; white-space: nowrap; }
.seg button.on { background: var(--fill-on); color: #fff; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3); }

.switch { width: 30px; height: 18px; border-radius: 9px; background: rgba(255, 255, 255, 0.18); position: relative; flex-shrink: 0; }
.switch::after { content: ''; position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: #fff; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.35); transition: left 0.15s; }
.switch.on { background: var(--accent); }
.switch.on::after { left: 14px; }

kbd { font: 600 10px Inter; color: #fff; background: rgba(255, 255, 255, 0.14); border-radius: 5px; padding: 2px 6px; }
.lbl { font: 600 10px Inter; letter-spacing: 0.05em; text-transform: uppercase; color: var(--text-3); margin-bottom: 6px; }
```

- [ ] **Step 3: Write `src/shared/ipc.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';

/** Rectangles are in image pixels of one monitor frame. */
export type Rect = { x: number; y: number; w: number; h: number };
export type WinRect = Rect & { id: number; title: string; app: string };
export type OverlayOptions = {
  timer_secs: number;
  show_thumbnail: boolean;
  remember_selection: boolean;
  show_pointer: boolean;
  screenshots_dir: string;
};
export type OverlayInfo = {
  token: number;
  mode: 'area' | 'window' | 'screen';
  index: number;
  active: boolean;
  width: number;
  height: number;
  windows: WinRect[];
  selection: Rect | null;
  hints: boolean;
  options: OverlayOptions;
};
export type Target = { kind: 'area'; rect: Rect } | { kind: 'window'; id: number; rect: Rect } | { kind: 'screen' };

export const overlayInfo = () => invoke<OverlayInfo | null>('overlay_info');
export const overlayFrame = () => invoke<ArrayBuffer>('overlay_frame');
export const overlayReady = (token: number) => invoke<void>('overlay_ready', { token });
export const overlayActivate = (token: number) => invoke<void>('overlay_activate', { token });
export const overlayCancel = () => invoke<void>('overlay_cancel');
export const overlayCapture = (token: number, target: Target) => invoke<void>('overlay_capture', { token, target });
```

- [ ] **Step 4: Write `src/overlay/index.html` and `src/overlay/overlay.css`** (final versions; Task 7 brings them to life)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot overlay</title>
  </head>
  <body data-mode="area">
    <canvas id="frame"></canvas>
    <div id="shade"></div>
    <div id="winhl" hidden></div>
    <div id="winlabel" class="glass" hidden><b></b><small></small></div>
    <div id="sel" hidden>
      <span class="h" data-h="nw"></span><span class="h" data-h="n"></span><span class="h" data-h="ne"></span>
      <span class="h" data-h="w"></span><span class="h" data-h="e"></span>
      <span class="h" data-h="sw"></span><span class="h" data-h="s"></span><span class="h" data-h="se"></span>
      <div id="dim" class="glass"></div>
    </div>
    <div id="loupe" hidden><canvas width="110" height="110"></canvas></div>
    <div id="coord" class="glass" hidden></div>
    <div id="hint" class="glass" hidden>
      <div><kbd>drag</kbd><span>select area</span></div>
      <div><kbd>Space</kbd><span>window mode</span></div>
      <div><kbd>⏎</kbd><span>capture</span></div>
      <div><kbd>Esc</kbd><span>cancel</span></div>
    </div>
    <div id="bar" class="glass" hidden>
      <button class="btn" data-act="cancel" data-icon="i-x" title="Cancel (Esc)" aria-label="Cancel"></button>
      <span class="sep"></span>
      <button class="btn" data-mode="screen" data-icon="i-screen" title="Capture screen" aria-label="Capture screen"></button>
      <button class="btn" data-mode="window" data-icon="i-window" title="Capture window (Space)" aria-label="Capture window"></button>
      <button class="btn" data-mode="area" data-icon="i-area" title="Capture area" aria-label="Capture area"></button>
      <span class="sep"></span>
      <button class="opt" data-act="options" aria-haspopup="true">Options <svg class="ic sm" aria-hidden="true"><use href="#i-down" /></svg></button>
      <button class="b1" data-act="capture">Capture</button>
    </div>
    <div id="pop" class="glass" hidden></div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

```css
html, body { width: 100%; height: 100%; overflow: hidden; background: #000; }
body { cursor: crosshair; }
body[data-mode='window'], body[data-mode='screen'] { cursor: pointer; }

#frame { position: fixed; inset: 0; width: 100vw; height: 100vh; }
#shade { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.28); pointer-events: none; }

#sel { position: fixed; border: 1.5px solid rgba(255, 255, 255, 0.95); box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0.5); pointer-events: none; }
#sel .h { position: absolute; width: 9px; height: 9px; margin: -5px 0 0 -5px; background: #fff; border-radius: 50%; box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.3); }
#sel .h[data-h='nw'] { left: 0; top: 0; }
#sel .h[data-h='n'] { left: 50%; top: 0; }
#sel .h[data-h='ne'] { left: 100%; top: 0; }
#sel .h[data-h='w'] { left: 0; top: 50%; }
#sel .h[data-h='e'] { left: 100%; top: 50%; }
#sel .h[data-h='sw'] { left: 0; top: 100%; }
#sel .h[data-h='s'] { left: 50%; top: 100%; }
#sel .h[data-h='se'] { left: 100%; top: 100%; }
#dim { position: absolute; top: calc(100% + 8px); right: 0; font: 500 11px Inter; padding: 3px 8px; border-radius: 6px; white-space: nowrap; }

#winhl { position: fixed; border-radius: 8px; background: rgba(10, 132, 255, 0.1); box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0.5), inset 0 0 0 2.5px var(--accent); pointer-events: none; }
#winlabel { position: fixed; transform: translate(-50%, -50%); display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 10px 16px; border-radius: 12px; font: 600 13px Inter; pointer-events: none; max-width: 60vw; }
#winlabel b { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%; }
#winlabel small { font: 500 11px Inter; color: var(--text-2); }

#loupe { position: fixed; width: 114px; height: 114px; border-radius: 50%; overflow: hidden; border: 2px solid #fff; box-shadow: 0 8px 26px rgba(0, 0, 0, 0.55); pointer-events: none; background: #000; }
#loupe canvas { display: block; }
#loupe::before { content: ''; position: absolute; inset: 0; background: repeating-linear-gradient(90deg, rgba(0, 0, 0, 0.12) 0 1px, transparent 1px 10px), repeating-linear-gradient(0deg, rgba(0, 0, 0, 0.12) 0 1px, transparent 1px 10px); }
#loupe::after { content: ''; position: absolute; left: 50px; top: 50px; width: 10px; height: 10px; box-shadow: 0 0 0 1.5px var(--accent); }
#coord { position: fixed; font: 500 10.5px Inter; padding: 3px 8px; border-radius: 6px; white-space: nowrap; pointer-events: none; }
#coord i { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin: 0 4px 0 6px; }

#hint { position: fixed; top: 6vh; left: 50%; transform: translateX(-50%); display: flex; gap: 14px; align-items: center; padding: 7px 14px; border-radius: 10px; font: 500 11.5px Inter; white-space: nowrap; pointer-events: none; }
#hint span { color: rgba(255, 255, 255, 0.6); }
#hint kbd { margin-right: 5px; }

#bar { position: fixed; left: 50%; bottom: 4.5vh; transform: translateX(-50%); display: flex; align-items: center; gap: 2px; padding: 6px; border-radius: 14px; white-space: nowrap; }
#bar .sep { width: 1px; height: 20px; background: rgba(255, 255, 255, 0.15); margin: 0 6px; }
#bar .opt { font: 500 12px Inter; padding: 0 10px; height: 30px; display: flex; align-items: center; gap: 5px; border-radius: 8px; color: #e5e5ea; }
#bar .opt:hover, #bar .opt.on { background: rgba(255, 255, 255, 0.17); }
#bar .b1 { margin-left: 4px; }

#pop { position: fixed; left: 50%; bottom: calc(4.5vh + 52px); transform: translateX(-50%); width: 430px; padding: 14px; border-radius: 14px; font: 500 12px Inter; display: grid; grid-template-columns: 1fr 1fr; gap: 14px 18px; background: var(--glass-strong); }
#pop .full { grid-column: 1 / -1; height: 1px; background: rgba(255, 255, 255, 0.1); margin: -4px 0; }
#pop .field { width: 100%; display: flex; align-items: center; gap: 7px; height: 28px; padding: 0 9px; border-radius: 7px; background: var(--fill); border: 1px solid var(--fill); }
#pop .field .grow { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left; }
#pop .toggles { grid-column: 1 / -1; display: grid; grid-template-columns: 1fr 1fr; gap: 0 18px; }
#pop .tog { display: flex; align-items: center; justify-content: space-between; height: 26px; gap: 10px; }
```

- [ ] **Step 5: Write the minimal `src/overlay/main.ts`**

This is just enough for the performance gate in Task 6. Task 7 replaces the whole file.

```ts
import '../shared/glass.css';
import './overlay.css';
import { listen } from '@tauri-apps/api/event';
import * as ipc from '../shared/ipc';

// Minimal overlay for the Task 6 performance gate; Task 7 replaces this file.
const canvas = document.querySelector<HTMLCanvasElement>('#frame')!;
let info: ipc.OverlayInfo | null = null;

async function load() {
  const next = await ipc.overlayInfo();
  if (!next) return;
  const buf = await ipc.overlayFrame();
  info = next;
  canvas.width = next.width;
  canvas.height = next.height;
  canvas.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(buf), next.width, next.height), 0, 0);
  await ipc.overlayReady(next.token);
}

addEventListener('keydown', (e) => {
  if (!info) return;
  if (e.key === 'Escape') void ipc.overlayCancel();
  if (e.key === 'Enter') void ipc.overlayCapture(info.token, { kind: 'screen' });
});
void listen('overlay:show', () => void load());
void listen('overlay:hide', () => {
  info = null;
  canvas.width = canvas.height = 0;
});
void load();
```

- [ ] **Step 6: Add the overlay window helpers to `ui.rs`**

Add these imports and functions:

```rust
use crate::{capture::Frame, err};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub fn overlay_index(label: &str) -> Option<usize> {
    label.strip_prefix("overlay-")?.parse().ok()
}

fn overlay_window(app: &AppHandle, i: usize) -> tauri::Result<WebviewWindow> {
    let label = format!("overlay-{i}");
    if let Some(w) = app.get_webview_window(&label) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("overlay/index.html".into()))
        .title("rshot overlay")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .build()
}

/// One hidden overlay per monitor, loaded ahead of time so Print feels instant.
pub fn ensure_overlays(app: &AppHandle) -> tauri::Result<()> {
    let n = xcap::Monitor::all().map(|m| m.len()).unwrap_or(1).max(1);
    for i in 0..n {
        overlay_window(app, i)?;
    }
    Ok(())
}

/// Puts overlay i fullscreen on frame i's monitor (only when it isn't there already).
pub fn place_overlays(app: &AppHandle, frames: &[Frame]) -> Result<(), String> {
    for (i, f) in frames.iter().enumerate() {
        let w = overlay_window(app, i).map_err(err)?;
        let pos = PhysicalPosition::new(f.x, f.y);
        if w.outer_position().ok() != Some(pos) || !w.is_fullscreen().unwrap_or(false) {
            w.set_fullscreen(false).map_err(err)?;
            w.set_position(pos).map_err(err)?;
            w.set_size(PhysicalSize::new(f.image.width(), f.image.height())).map_err(err)?;
            w.set_fullscreen(true).map_err(err)?;
        }
    }
    Ok(())
}

pub fn hide_overlays(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = w.hide();
        }
    }
    let _ = app.emit("overlay:hide", ());
}
```

- [ ] **Step 7: Write `overlay.rs`**

```rust
//! Overlay session: frozen frames per monitor, the frame IPC, and capture targets.

use crate::{
    capture::{self, Frame, WinRect},
    err, pipeline, store, ui, AppState,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::Ordering,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Response, AppHandle, Emitter, Manager, State, WebviewWindow};

pub struct Session {
    pub token: u64,
    pub mode: String,
    pub frames: Vec<Frame>,
    pub active: usize,
    pub started: Instant,
}

/// Image pixels of one frame.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Area { rect: Rect },
    Window { id: u32, rect: Rect },
    Screen,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OverlayOptions {
    pub timer_secs: u8,
    pub show_thumbnail: bool,
    pub remember_selection: bool,
    pub show_pointer: bool,
    pub screenshots_dir: String,
}

impl OverlayOptions {
    pub fn from_config(c: &store::Config) -> Self {
        Self {
            timer_secs: c.timer_secs,
            show_thumbnail: c.show_thumbnail,
            remember_selection: c.remember_selection,
            show_pointer: c.show_pointer,
            screenshots_dir: store::screenshots_dir(c).display().to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct OverlayInfo {
    token: u64,
    mode: String,
    index: usize,
    active: bool,
    width: u32,
    height: u32,
    windows: Vec<WinRect>,
    selection: Option<Rect>,
    hints: bool,
    options: OverlayOptions,
}

/// The hint bar shows on the first few overlays only.
const HINT_SESSIONS: u32 = 5;

fn epoch_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Grabs every monitor and asks each overlay page to show its frame.
pub fn start(app: &AppHandle, mode: &str) -> Result<(), String> {
    let started = Instant::now();
    eprintln!("rshot: overlay requested at {}", epoch_ms());
    let state = app.state::<AppState>();
    let show_pointer = state.config.lock().unwrap().show_pointer;
    let frames = capture::grab_all(show_pointer)?;
    eprintln!("rshot: grabbed {} monitor(s) in {} ms", frames.len(), started.elapsed().as_millis());
    let pos = app.cursor_position().map_err(err)?;
    let active = capture::frame_at(&frames, pos.x as i32, pos.y as i32);
    ui::place_overlays(app, &frames)?;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    *state.session.lock().unwrap() = Some(Session { token, mode: mode.into(), frames, active, started });
    {
        let mut c = state.config.lock().unwrap();
        if c.hints_shown <= HINT_SESSIONS {
            c.hints_shown += 1;
            let _ = store::save_config(&c);
        }
    }
    app.emit("overlay:show", token).map_err(err)
}

#[tauri::command]
pub fn overlay_info(window: WebviewWindow, state: State<'_, AppState>) -> Option<OverlayInfo> {
    let index = ui::overlay_index(window.label())?;
    let session = state.session.lock().unwrap();
    let s = session.as_ref()?;
    let f = s.frames.get(index)?;
    let c = state.config.lock().unwrap();
    let selection = c
        .last_selection
        .as_ref()
        .filter(|l| c.remember_selection && l.monitor == f.name)
        .map(|l| Rect { x: l.x.into(), y: l.y.into(), w: l.w.into(), h: l.h.into() });
    Some(OverlayInfo {
        token: s.token,
        mode: s.mode.clone(),
        index,
        active: index == s.active,
        width: f.image.width(),
        height: f.image.height(),
        windows: capture::windows_on(f),
        selection,
        hints: c.hints_shown <= HINT_SESSIONS,
        options: OverlayOptions::from_config(&c),
    })
}

/// Raw RGBA of this overlay's frame (width/height come from overlay_info).
#[tauri::command]
pub async fn overlay_frame(app: AppHandle, window: WebviewWindow) -> Result<Response, String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    let state = app.state::<AppState>();
    let session = state.session.lock().unwrap();
    let f = session.as_ref().and_then(|s| s.frames.get(index)).ok_or("no frame")?;
    Ok(Response::new(f.image.as_raw().clone()))
}

/// The page painted its frame: show it (and focus the one under the pointer).
#[tauri::command]
pub fn overlay_ready(window: WebviewWindow, state: State<'_, AppState>, token: u64) -> Result<(), String> {
    let session = state.session.lock().unwrap();
    let Some(s) = session.as_ref().filter(|s| s.token == token) else {
        return Ok(());
    };
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    window.show().map_err(err)?;
    if index == s.active {
        window.set_focus().map_err(err)?;
        eprintln!("rshot: overlay visible at {} ({} ms after trigger)", epoch_ms(), s.started.elapsed().as_millis());
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_activate(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>, token: u64) -> Result<(), String> {
    let index = ui::overlay_index(window.label()).ok_or("not an overlay")?;
    if let Some(s) = state.session.lock().unwrap().as_mut().filter(|s| s.token == token) {
        s.active = index;
    }
    let _ = window.set_focus();
    app.emit("overlay:active", index).map_err(err)
}

#[tauri::command]
pub fn overlay_cancel(app: AppHandle, state: State<'_, AppState>) {
    state.session.lock().unwrap().take();
    ui::hide_overlays(&app);
}

#[tauri::command]
pub async fn overlay_capture(app: AppHandle, window: WebviewWindow, token: u64, target: Target) -> Result<(), String> {
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
    let result = capture_from(&app, session, index, target);
    if let Err(e) = &result {
        pipeline::notify(&app, e);
    }
    result
}

fn capture_from(app: &AppHandle, session: Session, index: usize, target: Target) -> Result<(), String> {
    let frame = session.frames.into_iter().nth(index).ok_or("that monitor is gone")?;
    if let Target::Area { rect } = target {
        remember(app, &frame.name, rect);
    }
    let (w, h) = frame.image.dimensions();
    let clamp = |r: Rect| capture::clamp_rect(r.x, r.y, r.w, r.h, w, h).ok_or_else(|| "empty selection".to_string());
    let img = match target {
        Target::Screen => frame.image,
        Target::Area { rect } => capture::crop(&frame.image, clamp(rect)?),
        // Window pixels come from the window itself (even if covered); fall back to the frozen frame.
        Target::Window { id, rect } => match capture::window_image(id) {
            Ok(img) => img,
            Err(_) => capture::crop(&frame.image, clamp(rect)?),
        },
    };
    pipeline::finish_capture(app, img).map(|_| ())
}

fn remember(app: &AppHandle, monitor: &str, r: Rect) {
    let state = app.state::<AppState>();
    let mut c = state.config.lock().unwrap();
    c.last_selection = Some(store::Selection {
        monitor: monitor.into(),
        x: r.x.max(0.0) as u32,
        y: r.y.max(0.0) as u32,
        w: r.w.max(0.0) as u32,
        h: r.h.max(0.0) as u32,
    });
    let _ = store::save_config(&c);
}
```

- [ ] **Step 8: Wire `main.rs`**

- Add `mod overlay;`.
- Add these fields to `AppState`:

  ```rust
  pub session: std::sync::Mutex<Option<overlay::Session>>,
  pub next_token: std::sync::atomic::AtomicU64,
  ```

  and initialise them in `new()`:

  ```rust
  session: std::sync::Mutex::new(None),
  next_token: std::sync::atomic::AtomicU64::new(1),
  ```

- Add `.invoke_handler(tauri::generate_handler![...])` before `.setup(…)`:

  ```rust
  .invoke_handler(tauri::generate_handler![
      overlay::overlay_info,
      overlay::overlay_frame,
      overlay::overlay_ready,
      overlay::overlay_activate,
      overlay::overlay_cancel,
      overlay::overlay_capture,
  ])
  ```

- In `setup`, call `ui::ensure_overlays(app.handle())?;` before `dispatch(…)`.
- In `dispatch`, change the `CaptureArea` arm to `CaptureArea => overlay::start(app, "area"),`.

- [ ] **Step 9: Verify the plumbing**

```bash
npm run build && npm run tauri dev > /tmp/rshot-dev.log 2>&1 &   # wait for "dispatch Daemon"
./src-tauri/target/debug/rshot capture area
```

Expected:
- Every monitor shows its own frozen screenshot fullscreen, with no GNOME top bar visible.
- `grep "overlay visible" /tmp/rshot-dev.log` prints one line with a millisecond figure.
- Press `Esc`: all overlays disappear.
- Run it again and press `Enter`: a full-screen PNG of the monitor under the pointer is saved and copied (check with `xclip -selection clipboard -o`).

If any overlay is on the wrong monitor, or doesn't cover the top bar, stop and report the log. Task 6 relies on this working.

- [ ] **Step 10: Commit**

```bash
git add -A src src-tauri vite.config.ts && git commit -m "feat: preloaded per-monitor overlays with frozen frames over binary IPC

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Performance and focus gate (spec §5): measure, record, fix or escalate

**Files:**
- Create: `docs/perf.md`
- Modify (only if a check fails): `src-tauri/src/ui.rs`, `src-tauri/Cargo.toml`

**Interfaces:**
- Produces: `docs/perf.md` with measured numbers and the decision. There's no code interface unless a fallback below is applied.

- [ ] **Step 1: Build a release binary and start it**

```bash
npm run tauri build -- --no-bundle
pkill -x rshot; ./src-tauri/target/release/rshot 2> /tmp/rshot-perf.log &
sleep 3
```

- [ ] **Step 2: Add a temporary GNOME keybinding (`Ctrl+Super+F12`) that runs the release binary**

This exercises the same path GNOME's `Print` will use, without touching `Print`.

```bash
ORIG=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings); echo "$ORIG" > /tmp/rshot-orig-custom
P=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rshot-perf/
S="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$P"
gsettings set "$S" name "'rshot perf'"
gsettings set "$S" command "'$PWD/src-tauri/target/release/rshot capture area'"
gsettings set "$S" binding "'<Ctrl><Super>F12'"
python3 - "$ORIG" "$P" <<'EOF'
import ast, subprocess, sys
cur = [] if sys.argv[1].startswith('@as') else ast.literal_eval(sys.argv[1])
subprocess.run(['gsettings', 'set', 'org.gnome.settings-daemon.plugins.media-keys', 'custom-keybindings', str(cur + [sys.argv[2]])], check=True)
EOF
```

- [ ] **Step 3: Measure latency and focus (5 runs)**

```bash
for i in 1 2 3 4 5; do
  T0=$(date +%s%3N); xdotool key ctrl+super+F12; sleep 1.5
  ACTIVE=$(xdotool getactivewindow getwindowname)
  xdotool key Escape; sleep 0.8
  VIS=$(grep "overlay visible at" /tmp/rshot-perf.log | tail -1 | sed -E 's/.*at ([0-9]+).*/\1/')
  echo "run $i: key→visible $((VIS - T0)) ms, focused='$ACTIVE', still visible after Esc: $(xdotool search --onlyvisible --name 'rshot overlay' | wc -l)"
done
```

Pass criteria:
- `key→visible` ≤ 250 ms on every run.
- `focused='rshot overlay'`.
- `still visible after Esc: 0`.

- [ ] **Step 4: Measure idle memory (daemon + WebKit children)**

```bash
PID=$(pgrep -x rshot); ps -o rss= -p "$PID" --ppid "$PID" | awk '{s+=$1} END {printf "%.0f MB\n", s/1024}'
```

Pass: ≤ 200 MB.

- [ ] **Step 5: Remove the temporary keybinding (always, pass or fail)**

```bash
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$(cat /tmp/rshot-orig-custom)"
gsettings reset-recursively "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rshot-perf/"
gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings   # must equal the saved original
```

- [ ] **Step 6: Apply the matching fallback only for a failed check, then repeat Steps 1–5**

- **Focus fails** (`focused` isn't the overlay, or Esc does nothing): GNOME's focus-stealing prevention blocked the present. Stamp the X server time before presenting.

  Add these Linux-only dependencies, which match Tauri's gtk 0.18:

  ```bash
  cargo add gtk@0.18 gdkx11@0.18 --target 'cfg(target_os = "linux")'
  ```

  In `ui.rs`:

  ```rust
  /// Presents a window with a fresh X server timestamp so Mutter grants it focus.
  #[cfg(target_os = "linux")]
  pub fn force_focus(w: &WebviewWindow) {
      let w2 = w.clone();
      let _ = w.run_on_main_thread(move || {
          use gtk::prelude::*;
          if let Ok(gw) = w2.gtk_window() {
              if let Some(gdk) = gw.window() {
                  if let Ok(x11) = gdk.downcast::<gdkx11::X11Window>() {
                      let t = gdkx11::x11_get_server_time(&x11);
                      x11.set_user_time(t);
                      gw.present_with_time(t);
                  }
              }
          }
      });
  }
  ```

  Then call `#[cfg(target_os = "linux")] ui::force_focus(&window);` right after `window.set_focus()` in `overlay_ready`. (If a `gdkx11` function name doesn't compile, look it up on docs.rs for gdkx11 0.18. The functions are the GDK X11 helpers `gdk_x11_get_server_time` and `gdk_x11_window_set_user_time`.)
- **Memory over 200 MB:** stop preloading. Remove the `ensure_overlays` call from `setup`. In `hide_overlays`, call `w.destroy()` instead of `w.hide()`. Re-run Step 3: latency must still be ≤ 250 ms.
- **Latency over 250 ms:** don't guess. Record the `grabbed … ms` and `overlay visible … ms after trigger` lines from `/tmp/rshot-perf.log` for each run in `docs/perf.md`. Then **stop with status BLOCKED** and report the breakdown (grab time vs IPC+paint vs launch). The controller decides between PNG frames, downscaled preview frames, or a native overlay.

- [ ] **Step 7: Write `docs/perf.md`**

```markdown
# Performance gate (spec §5) — <date>

Machine: Ubuntu 24.04, GNOME 46, X11; monitors DP-1 1920×1080, DP-4 2560×1600, HDMI-0 1920×1080.
Build: release, preloaded overlays.

| Run | key → visible (ms) | focus OK | hidden after Esc |
|---|---|---|---|
| 1 | … | … | … |
| … | | | |

Idle RSS (daemon + WebKit): … MB
Grab time (3 monitors): … ms

Decision: <pass as-is | applied force_focus | switched to on-demand overlays | BLOCKED: …>
```

- [ ] **Step 8: Commit**

```bash
git add docs/perf.md src-tauri && git commit -m "perf: measure overlay latency, focus and memory against spec targets

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Overlay UI — selection, handles, modes, loupe, hints, multi-monitor

**Files:**
- Create: `src/shared/icons.ts`, `src/overlay/selection.ts`, `src/overlay/selection.test.ts`
- Replace: `src/overlay/main.ts`

**Interfaces:**
- Consumes: the `ipc.ts` overlay functions from Task 5.
- Produces: `selection.ts` exports:

  ```ts
  type Rect
  type Handle = 'n'|'s'|'e'|'w'|'ne'|'nw'|'se'|'sw'
  fromPoints(ax, ay, bx, by, square?): Rect
  clamp(r, W, H): Rect
  move(r, dx, dy, W, H): Rect
  resize(r, handle, px, py): Rect
  handleAt(r, px, py, tol): Handle | 'inside' | null
  windowAt(wins, px, py)
  ```

  `icons.ts` exports `mountIcons()` and `SPRITE`. `main.ts` holds four local placeholders (`optionsOpen`, `closeOptions`, `toggleOptions`, `renderOptions`) that Task 8 replaces with an import.

- [ ] **Step 1: Write the failing geometry tests**

`src/overlay/selection.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { clamp, fromPoints, handleAt, move, resize, windowAt } from './selection';

describe('fromPoints', () => {
  it('normalizes a drag up-left', () => expect(fromPoints(100, 80, 40, 20)).toEqual({ x: 40, y: 20, w: 60, h: 60 }));
  it('squares from the anchor', () => expect(fromPoints(10, 10, 50, 30, true)).toEqual({ x: 10, y: 10, w: 40, h: 40 }));
  it('squares up-left', () => expect(fromPoints(50, 50, 40, 20, true)).toEqual({ x: 20, y: 20, w: 30, h: 30 }));
});

describe('clamp', () => {
  it('clips to the frame', () => expect(clamp({ x: -10, y: 90, w: 50, h: 50 }, 100, 100)).toEqual({ x: 0, y: 90, w: 40, h: 10 }));
});

describe('move', () => {
  it('keeps the rect inside the frame', () => {
    expect(move({ x: 10, y: 10, w: 20, h: 20 }, 100, -50, 100, 100)).toEqual({ x: 80, y: 0, w: 20, h: 20 });
  });
});

describe('resize', () => {
  const r = { x: 10, y: 10, w: 40, h: 30 };
  it('drags the south-east corner', () => expect(resize(r, 'se', 70, 60)).toEqual({ x: 10, y: 10, w: 60, h: 50 }));
  it('drags the west edge past the east edge and flips', () => expect(resize(r, 'w', 80, 999)).toEqual({ x: 50, y: 10, w: 30, h: 30 }));
});

describe('handleAt', () => {
  const r = { x: 100, y: 100, w: 200, h: 100 };
  it('finds corners and edges within tolerance', () => {
    expect(handleAt(r, 103, 98, 8)).toBe('nw');
    expect(handleAt(r, 200, 205, 8)).toBe('s');
    expect(handleAt(r, 297, 150, 8)).toBe('e');
  });
  it('reports inside and outside', () => {
    expect(handleAt(r, 200, 150, 8)).toBe('inside');
    expect(handleAt(r, 50, 50, 8)).toBeNull();
  });
});

describe('windowAt', () => {
  it('returns the first (top-most) window containing the point', () => {
    const wins = [{ id: 2, x: 50, y: 50, w: 100, h: 100 }, { id: 1, x: 0, y: 0, w: 500, h: 500 }];
    expect(windowAt(wins, 60, 60)?.id).toBe(2);
    expect(windowAt(wins, 10, 10)?.id).toBe(1);
    expect(windowAt(wins, 600, 10)).toBeNull();
  });
});
```

`src/overlay/selection.ts` (stubs):

```ts
export type Rect = { x: number; y: number; w: number; h: number };
export type Handle = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

export function fromPoints(_ax: number, _ay: number, _bx: number, _by: number, _square = false): Rect {
  return { x: 0, y: 0, w: 0, h: 0 };
}
export function clamp(r: Rect, _W: number, _H: number): Rect {
  return r;
}
export function move(r: Rect, _dx: number, _dy: number, _W: number, _H: number): Rect {
  return r;
}
export function resize(r: Rect, _h: Handle, _px: number, _py: number): Rect {
  return r;
}
export function handleAt(_r: Rect, _px: number, _py: number, _tol: number): Handle | 'inside' | null {
  return null;
}
export function windowAt<T extends Rect>(_wins: T[], _px: number, _py: number): T | null {
  return null;
}
```

Run: `npm test`
Expected: FAIL. Every `describe` has failing cases.

- [ ] **Step 2: Implement `selection.ts`**

```ts
/** Pure geometry for the overlay. All values are image pixels of one monitor frame. */
export type Rect = { x: number; y: number; w: number; h: number };
export type Handle = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

/** Rect spanned by a drag from (ax, ay) to (bx, by); `square` keeps it 1:1 from the anchor. */
export function fromPoints(ax: number, ay: number, bx: number, by: number, square = false): Rect {
  let w = bx - ax;
  let h = by - ay;
  if (square) {
    const s = Math.max(Math.abs(w), Math.abs(h));
    w = Math.sign(w || 1) * s;
    h = Math.sign(h || 1) * s;
  }
  return { x: Math.min(ax, ax + w), y: Math.min(ay, ay + h), w: Math.abs(w), h: Math.abs(h) };
}

export function clamp(r: Rect, W: number, H: number): Rect {
  const x = Math.max(0, Math.min(r.x, W));
  const y = Math.max(0, Math.min(r.y, H));
  return { x, y, w: Math.max(0, Math.min(r.x + r.w, W) - x), h: Math.max(0, Math.min(r.y + r.h, H) - y) };
}

export function move(r: Rect, dx: number, dy: number, W: number, H: number): Rect {
  return { ...r, x: Math.max(0, Math.min(r.x + dx, W - r.w)), y: Math.max(0, Math.min(r.y + dy, H - r.h)) };
}

/** Moves the edges named by `h` to the pointer, then normalizes (edges may cross). */
export function resize(r: Rect, h: Handle, px: number, py: number): Rect {
  let x0 = r.x;
  let y0 = r.y;
  let x1 = r.x + r.w;
  let y1 = r.y + r.h;
  if (h.includes('w')) x0 = px;
  if (h.includes('e')) x1 = px;
  if (h.includes('n')) y0 = py;
  if (h.includes('s')) y1 = py;
  return fromPoints(x0, y0, x1, y1);
}

export function handleAt(r: Rect, px: number, py: number, tol: number): Handle | 'inside' | null {
  const near = (a: number, b: number) => Math.abs(a - b) <= tol;
  if (px < r.x - tol || px > r.x + r.w + tol || py < r.y - tol || py > r.y + r.h + tol) return null;
  const v = near(py, r.y) ? 'n' : near(py, r.y + r.h) ? 's' : '';
  const hz = near(px, r.x) ? 'w' : near(px, r.x + r.w) ? 'e' : '';
  if (v || hz) return (v + hz) as Handle;
  return px > r.x && px < r.x + r.w && py > r.y && py < r.y + r.h ? 'inside' : null;
}

/** First window containing the point; callers pass windows top-most first. */
export function windowAt<T extends Rect>(wins: T[], px: number, py: number): T | null {
  return wins.find((w) => px >= w.x && py >= w.y && px < w.x + w.w && py < w.y + w.h) ?? null;
}
```

Run: `npm test`
Expected: `10 passed`.

- [ ] **Step 3: Write `src/shared/icons.ts`** (the icon paths come from the mockups)

```ts
export const SPRITE = `<svg xmlns="http://www.w3.org/2000/svg" style="display:none">
<symbol id="i-x" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></symbol>
<symbol id="i-screen" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/></symbol>
<symbol id="i-window" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 9h18"/></symbol>
<symbol id="i-area" viewBox="0 0 24 24"><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3"/></symbol>
<symbol id="i-down" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6"/></symbol>
<symbol id="i-folder" viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></symbol>
<symbol id="i-trash" viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/></symbol>
<symbol id="i-check" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5"/></symbol>
<symbol id="i-logo" viewBox="0 0 24 24"><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3"/><circle cx="12" cy="12" r="3"/></symbol>
</svg>`;

/** Inserts the sprite and fills every `[data-icon]` element with its icon. */
export function mountIcons(): void {
  document.body.insertAdjacentHTML('afterbegin', SPRITE);
  document.querySelectorAll<HTMLElement>('[data-icon]').forEach((el) => {
    el.insertAdjacentHTML('afterbegin', `<svg class="ic" aria-hidden="true"><use href="#${el.dataset.icon}"/></svg>`);
  });
}
```

- [ ] **Step 4: Replace `src/overlay/main.ts`**

```ts
import '../shared/glass.css';
import './overlay.css';
import { listen } from '@tauri-apps/api/event';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import { type Handle, type Rect, clamp, fromPoints, handleAt, move, resize, windowAt } from './selection';

// Options popover arrives in Task 8.
const optionsOpen = () => false;
const closeOptions = () => {};
const toggleOptions = () => {};
const renderOptions = (_o: ipc.OverlayOptions) => {};

mountIcons();

const q = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const canvas = q<HTMLCanvasElement>('#frame');
const selEl = q('#sel');
const dimEl = q('#dim');
const winhl = q('#winhl');
const winlabel = q('#winlabel');
const loupe = q('#loupe');
const loupeCtx = q<HTMLCanvasElement>('#loupe canvas').getContext('2d')!;
const coord = q('#coord');
const hint = q('#hint');
const bar = q('#bar');
const shade = q('#shade');

type Mode = ipc.OverlayInfo['mode'];
type Drag = { kind: 'new' | 'move' | Handle; ax: number; ay: number; start: Rect | null };

let info: ipc.OverlayInfo | null = null;
let pixels: Uint8ClampedArray | null = null;
let mode: Mode = 'area';
let sel: Rect | null = null; // image pixels
let drag: Drag | null = null;
let pointer: [number, number] | null = null; // image pixels
let busy = false;

/** Image pixels per CSS pixel (monitor scale); read live because the window size settles after show. */
const k = () => (info ? info.width / innerWidth : 1);
const toImg = (e: MouseEvent): [number, number] => [e.clientX * k(), e.clientY * k()];
const place = (el: HTMLElement, r: Rect) => {
  const s = k();
  Object.assign(el.style, { left: `${r.x / s}px`, top: `${r.y / s}px`, width: `${r.w / s}px`, height: `${r.h / s}px` });
};

async function load() {
  const next = await ipc.overlayInfo();
  if (!next) return;
  const buf = await ipc.overlayFrame();
  info = next;
  pixels = new Uint8ClampedArray(buf);
  canvas.width = next.width;
  canvas.height = next.height;
  canvas.getContext('2d')!.putImageData(new ImageData(pixels, next.width, next.height), 0, 0);
  mode = next.mode;
  sel = next.selection;
  drag = null;
  pointer = null;
  busy = false;
  renderOptions(next.options);
  render();
  await ipc.overlayReady(next.token);
}

function highlight(): { rect: Rect; title: string; id?: number } | null {
  if (!info) return null;
  if (mode === 'screen') return { rect: { x: 0, y: 0, w: info.width, h: info.height }, title: 'Screen' };
  if (mode !== 'window' || !pointer) return null;
  const w = windowAt(info.windows, pointer[0], pointer[1]);
  return w ? { rect: clamp(w, info.width, info.height), title: w.title || w.app, id: w.id } : null;
}

function render() {
  if (!info) return;
  document.body.dataset.mode = mode;
  bar.hidden = !info.active;
  hint.hidden = !(info.active && info.hints);
  bar.querySelectorAll<HTMLElement>('[data-mode]').forEach((b) => b.classList.toggle('on', b.dataset.mode === mode));
  shade.hidden = mode !== 'area' || !!sel;
  selEl.hidden = mode !== 'area' || !sel;
  if (sel) {
    place(selEl, sel);
    dimEl.textContent = `${Math.round(sel.w)} × ${Math.round(sel.h)}`;
  }
  const t = info.active || mode === 'window' ? highlight() : null;
  winhl.hidden = winlabel.hidden = !t;
  if (t) {
    place(winhl, t.rect);
    const s = k();
    Object.assign(winlabel.style, { left: `${(t.rect.x + t.rect.w / 2) / s}px`, top: `${(t.rect.y + t.rect.h / 2) / s}px` });
    winlabel.querySelector('b')!.textContent = t.title;
    winlabel.querySelector('small')!.textContent = `${Math.round(t.rect.w)} × ${Math.round(t.rect.h)} · click to capture`;
  }
  renderLoupe();
}

function renderLoupe() {
  const show = !!info && !!pixels && !!pointer && info.active && mode === 'area' && drag?.kind !== 'move';
  loupe.hidden = coord.hidden = !show;
  if (!show || !info || !pixels || !pointer) return;
  const x = Math.min(info.width - 1, Math.max(0, Math.floor(pointer[0])));
  const y = Math.min(info.height - 1, Math.max(0, Math.floor(pointer[1])));
  loupeCtx.imageSmoothingEnabled = false;
  loupeCtx.fillStyle = '#000';
  loupeCtx.fillRect(0, 0, 110, 110);
  loupeCtx.drawImage(canvas, x - 5, y - 5, 11, 11, 0, 0, 110, 110);
  const i = (y * info.width + x) * 4;
  const hex = '#' + [pixels[i], pixels[i + 1], pixels[i + 2]].map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase();
  coord.innerHTML = `${x}, ${y}<i style="background:${hex}"></i>${hex}`;
  const s = k();
  const cx = pointer[0] / s;
  const cy = pointer[1] / s;
  const lx = cx + 18 + 114 > innerWidth ? cx - 18 - 114 : cx + 18;
  const ly = cy + 18 + 114 + 30 > innerHeight ? cy - 18 - 114 - 30 : cy + 18;
  Object.assign(loupe.style, { left: `${lx}px`, top: `${ly}px` });
  Object.assign(coord.style, { left: `${lx}px`, top: `${ly + 120}px` });
}

function setMode(m: Mode) {
  mode = m;
  closeOptions();
  render();
}

function captureNow() {
  if (!info) return;
  if (mode === 'screen') void capture({ kind: 'screen' });
  else if (mode === 'window') {
    const t = highlight();
    if (t?.id !== undefined) void capture({ kind: 'window', id: t.id, rect: t.rect });
  } else if (sel && sel.w >= 1 && sel.h >= 1) void capture({ kind: 'area', rect: sel });
}

async function capture(target: ipc.Target) {
  if (!info || busy) return;
  busy = true;
  // Rust hides the overlays, reports failures as a notification, and resets on the next show.
  await ipc.overlayCapture(info.token, target).catch(() => {});
}

addEventListener('mousedown', (e) => {
  if (!info || busy || e.button !== 0 || (e.target as Element).closest('#bar, #pop')) return;
  closeOptions();
  if (!info.active) {
    info.active = true;
    void ipc.overlayActivate(info.token);
  }
  pointer = toImg(e);
  const [x, y] = pointer;
  if (mode !== 'area') return captureNow();
  const h = sel ? handleAt(sel, x, y, 8 * k()) : null;
  drag =
    h === 'inside'
      ? { kind: 'move', ax: x, ay: y, start: sel }
      : h
        ? { kind: h, ax: x, ay: y, start: sel }
        : { kind: 'new', ax: x, ay: y, start: null };
  if (drag.kind === 'new') sel = null;
  render();
});

addEventListener('mousemove', (e) => {
  if (!info) return;
  pointer = toImg(e);
  const [x, y] = pointer;
  const W = info.width;
  const H = info.height;
  if (drag?.kind === 'new') sel = clamp(fromPoints(drag.ax, drag.ay, x, y, e.shiftKey), W, H);
  else if (drag?.kind === 'move' && drag.start) sel = move(drag.start, x - drag.ax, y - drag.ay, W, H);
  else if (drag?.start) sel = clamp(resize(drag.start, drag.kind as Handle, x, y), W, H);
  render();
});

addEventListener('mouseup', () => {
  if (drag?.kind === 'new' && sel && (sel.w < 3 || sel.h < 3)) sel = null;
  drag = null;
  render();
});

addEventListener('dblclick', (e) => {
  if (mode === 'area' && sel && handleAt(sel, ...toImg(e), 0) === 'inside') captureNow();
});

addEventListener('keydown', (e) => {
  if (!info || busy) return;
  if (e.key === 'Escape') {
    if (optionsOpen()) closeOptions();
    else void ipc.overlayCancel();
  } else if (e.key === 'Enter') captureNow();
  else if (e.key === ' ') {
    e.preventDefault();
    setMode(mode === 'window' ? 'area' : 'window');
  } else if (e.key.startsWith('Arrow') && sel && mode === 'area') {
    e.preventDefault();
    const d = e.shiftKey ? 10 : 1;
    const step: Record<string, [number, number]> = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] };
    const [dx, dy] = step[e.key] ?? [0, 0];
    sel = move(sel, dx, dy, info.width, info.height);
    render();
  }
});

bar.addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.mode) setMode(b.dataset.mode as Mode);
  else if (b.dataset.act === 'cancel') void ipc.overlayCancel();
  else if (b.dataset.act === 'capture') captureNow();
  else if (b.dataset.act === 'options') toggleOptions();
});

void listen<number>('overlay:show', () => void load());
void listen<number>('overlay:active', (e) => {
  if (!info) return;
  info.active = e.payload === info.index;
  if (!info.active) sel = null;
  render();
});
void listen('overlay:hide', () => {
  info = null;
  pixels = null;
  sel = null;
  drag = null;
  canvas.width = canvas.height = 0;
  closeOptions();
});
void load();
```

Run: `npm run build && npm test`
Expected: `tsc` reports no errors, `vite build` succeeds, and all tests pass.

- [ ] **Step 5: Verify interactively** (`npm run tauri dev`, then `./src-tauri/target/debug/rshot capture area`)

Compare with `docs/design/mockups/02-overlay-states.png` and check each item:
- The screen dims slightly. The hint bar is at the top and the toolbar at the bottom, only on the monitor under the pointer.
- The magnifier follows the crosshair, showing pixel coordinates and the hex colour.
- Dragging draws the selection with 8 round handles and a `W × H` label. Outside the selection is dimmed at 50%. Shift gives a square.
- The handles resize the selection, dragging inside moves it, and arrow keys nudge by 1 px (Shift+arrow by 10 px).
- `⏎` or a double-click inside captures the area. Check that the clipboard path's PNG has the size the label showed.
- Press `Space`: hovering a window tints it blue with a title and size label, and a click saves that window alone.
- The Screen button highlights the whole monitor, and a click captures it.
- Clicking on another monitor moves the toolbar and hint there.
- A second `rshot capture area` pre-draws the last selection, and `⏎` captures it straight away.

- [ ] **Step 6: Commit**

```bash
git add src && git commit -m "feat: Glass overlay with selection, window and screen modes, loupe and hints

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Options popover + timer countdown

**Files:**
- Create: `src/overlay/options.ts`, `src/countdown/index.html`, `src/countdown/main.ts`
- Modify: `src/overlay/main.ts` (swap in the import), `src/shared/ipc.ts`, `vite.config.ts`, `src-tauri/src/overlay.rs`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `store::dir_setting`, `OverlayOptions`.
- Produces:
  - Commands: `set_overlay_options(options)`, `pick_folder() -> Option<String>`, `countdown_info() -> u8`, `countdown_done()`, `countdown_cancel()`.
  - `overlay::Pending { monitor, target, secs }` and `AppState.pending: Mutex<Option<Pending>>`.
  - `ui::{popup, close_prefix, show_countdown}`.
  - TS: `ipc.setOverlayOptions`, `ipc.pickFolder`.

- [ ] **Step 1: Add the dialog plugin**

Run: `cargo add tauri-plugin-dialog@2.7`. In `main.rs`, add `.plugin(tauri_plugin_dialog::init())` after the notification plugin.

- [ ] **Step 2: Add popup helpers and the countdown window to `ui.rs`**

Popups get unique labels (`thumbnail-7`, `countdown-8`). A label can't be reused until destroy completes, and old ones are closed by prefix.

```rust
static POPUP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn close_prefix(app: &AppHandle, prefix: &str) {
    for (label, w) in app.webview_windows() {
        if label.starts_with(&format!("{prefix}-")) {
            let _ = w.destroy();
        }
    }
}

/// A small undecorated, transparent, always-on-top window (thumbnail, countdown).
pub fn popup(app: &AppHandle, prefix: &str, page: &str, w: f64, h: f64, focused: bool) -> Result<WebviewWindow, String> {
    close_prefix(app, prefix);
    let n = POPUP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    WebviewWindowBuilder::new(app, format!("{prefix}-{n}"), WebviewUrl::App(page.into()))
        .title("rshot")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(focused)
        .visible(false)
        .inner_size(w, h)
        .build()
        .map_err(err)
}

fn monitor_at(app: &AppHandle, x: f64, y: f64) -> Result<tauri::Monitor, String> {
    app.monitor_from_point(x, y)
        .map_err(err)?
        .or(app.primary_monitor().map_err(err)?)
        .ok_or_else(|| "no monitor".to_string())
}

/// Countdown ring centred on a desktop point (physical pixels).
pub fn show_countdown(app: &AppHandle, (cx, cy): (i32, i32)) -> Result<(), String> {
    let s = monitor_at(app, cx.into(), cy.into())?.scale_factor();
    let (w, h) = (160.0, 180.0);
    let win = popup(app, "countdown", "countdown/index.html", w, h, true)?;
    win.set_position(PhysicalPosition::new(cx - (w * s / 2.0) as i32, cy - (h * s / 2.0) as i32)).map_err(err)?;
    win.show().map_err(err)?;
    win.set_focus().map_err(err)
}
```

- [ ] **Step 3: Add options, folder picking and the timer to `overlay.rs`**

Add these new items:

```rust
/// A timed capture waiting for its countdown.
pub struct Pending {
    pub monitor: String,
    pub target: Target,
    pub secs: u8,
}

#[tauri::command]
pub fn set_overlay_options(state: State<'_, AppState>, options: OverlayOptions) -> Result<(), String> {
    let mut c = state.config.lock().unwrap();
    c.timer_secs = options.timer_secs;
    c.show_thumbnail = options.show_thumbnail;
    c.remember_selection = options.remember_selection;
    c.show_pointer = options.show_pointer;
    c.screenshots_dir = store::dir_setting(&options.screenshots_dir);
    store::save_config(&c).map_err(err)
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle, window: WebviewWindow) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_parent(&window)
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

#[tauri::command]
pub fn countdown_info(state: State<'_, AppState>) -> u8 {
    state.pending.lock().unwrap().as_ref().map_or(0, |p| p.secs)
}

#[tauri::command]
pub async fn countdown_done(app: AppHandle) -> Result<(), String> {
    let pending = app.state::<AppState>().pending.lock().unwrap().take();
    ui::close_prefix(&app, "countdown");
    let Some(p) = pending else { return Ok(()) };
    std::thread::sleep(std::time::Duration::from_millis(200)); // let the compositor drop the ring
    let result = run_pending(&app, p);
    if let Err(e) = &result {
        pipeline::notify(&app, e);
    }
    result
}

#[tauri::command]
pub fn countdown_cancel(app: AppHandle, state: State<'_, AppState>) {
    state.pending.lock().unwrap().take();
    ui::close_prefix(&app, "countdown");
}

/// After the countdown: grab the live screen again and cut the same target.
fn run_pending(app: &AppHandle, p: Pending) -> Result<(), String> {
    let img = match p.target {
        Target::Window { id, .. } => capture::window_image(id)?,
        target => {
            let show_pointer = app.state::<AppState>().config.lock().unwrap().show_pointer;
            let f = capture::grab_all(show_pointer)?
                .into_iter()
                .find(|f| f.name == p.monitor)
                .ok_or("that monitor is gone")?;
            match target {
                Target::Area { rect } => capture::crop(
                    &f.image,
                    capture::clamp_rect(rect.x, rect.y, rect.w, rect.h, f.image.width(), f.image.height())
                        .ok_or("empty selection")?,
                ),
                _ => f.image,
            }
        }
    };
    pipeline::finish_capture(app, img).map(|_| ())
}
```

In `capture_from`, add this directly after the `if let Target::Area { rect } = target { remember(…); }` block:

```rust
    let secs = app.state::<AppState>().config.lock().unwrap().timer_secs;
    if secs > 0 {
        let (fw, fh) = frame.image.dimensions();
        let center = match target {
            Target::Area { rect } | Target::Window { rect, .. } => {
                (frame.x + (rect.x + rect.w / 2.0) as i32, frame.y + (rect.y + rect.h / 2.0) as i32)
            }
            Target::Screen => (frame.x + (fw / 2) as i32, frame.y + (fh / 2) as i32),
        };
        *app.state::<AppState>().pending.lock().unwrap() = Some(Pending { monitor: frame.name, target, secs });
        return ui::show_countdown(app, center);
    }
```

In `main.rs`:
- Add `pub pending: std::sync::Mutex<Option<overlay::Pending>>,` to `AppState` and `pending: std::sync::Mutex::new(None),` to `new()`.
- Add `overlay::set_overlay_options, overlay::pick_folder, overlay::countdown_info, overlay::countdown_done, overlay::countdown_cancel,` to `generate_handler!`.

- [ ] **Step 4: Write `src/overlay/options.ts` and swap it into `main.ts`**

```ts
import * as ipc from '../shared/ipc';

type BoolKey = 'show_thumbnail' | 'remember_selection' | 'show_pointer';

const pop = document.querySelector<HTMLElement>('#pop')!;
const optionsButton = () => document.querySelector<HTMLElement>('#bar [data-act="options"]');
let opts: ipc.OverlayOptions | null = null;

pop.innerHTML = `
  <div><div class="lbl">Save to</div>
    <button class="field" data-act="folder"><svg class="ic" aria-hidden="true"><use href="#i-folder"/></svg><span class="grow"></span><svg class="ic sm" aria-hidden="true"><use href="#i-down"/></svg></button></div>
  <div><div class="lbl">Timer</div>
    <div class="seg">${[0, 3, 5, 10].map((n) => `<button data-timer="${n}">${n ? `${n} s` : 'Off'}</button>`).join('')}</div></div>
  <div class="full"></div>
  <div class="toggles">
    <label class="tog">Show floating thumbnail<button class="switch" role="switch" data-opt="show_thumbnail"></button></label>
    <label class="tog">Remember last selection<button class="switch" role="switch" data-opt="remember_selection"></button></label>
    <label class="tog">Show mouse pointer<button class="switch" role="switch" data-opt="show_pointer" title="Applies from the next capture"></button></label>
  </div>`;

export function renderOptions(o: ipc.OverlayOptions) {
  opts = o;
  pop.querySelector('.grow')!.textContent = o.screenshots_dir;
  pop.querySelectorAll<HTMLElement>('[data-timer]').forEach((b) => b.classList.toggle('on', Number(b.dataset.timer) === o.timer_secs));
  pop.querySelectorAll<HTMLElement>('[data-opt]').forEach((b) => {
    const on = o[b.dataset.opt as BoolKey];
    b.classList.toggle('on', on);
    b.setAttribute('aria-checked', String(on));
  });
}

export const optionsOpen = () => !pop.hidden;

export function closeOptions() {
  pop.hidden = true;
  optionsButton()?.classList.remove('on');
}

export function toggleOptions() {
  pop.hidden = !pop.hidden;
  optionsButton()?.classList.toggle('on', !pop.hidden);
}

pop.addEventListener('click', async (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b || !opts) return;
  if (b.dataset.timer) opts.timer_secs = Number(b.dataset.timer);
  else if (b.dataset.opt) {
    const key = b.dataset.opt as BoolKey;
    opts[key] = !opts[key];
  } else if (b.dataset.act === 'folder') {
    const dir = await ipc.pickFolder();
    if (!dir) return;
    opts.screenshots_dir = dir;
  }
  renderOptions(opts);
  await ipc.setOverlayOptions(opts);
});
```

In `src/overlay/main.ts`, replace the four placeholder lines (from `// Options popover arrives in Task 8.` through `const renderOptions = …`) with:

```ts
import { closeOptions, optionsOpen, renderOptions, toggleOptions } from './options';
```

Move that import up with the others.

Append to `src/shared/ipc.ts`:

```ts
export const setOverlayOptions = (options: OverlayOptions) => invoke<void>('set_overlay_options', { options });
export const pickFolder = () => invoke<string | null>('pick_folder');
```

- [ ] **Step 5: Write the countdown page**

`src/countdown/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot countdown</title>
    <style>
      html, body { height: 100%; overflow: hidden; }
      body { display: grid; place-items: center; }
      #ring { position: relative; width: 108px; height: 108px; border-radius: 50%; display: grid; place-items: center; font: 600 50px Inter; }
      #ring svg { position: absolute; inset: -2px; width: 112px; height: 112px; transform: rotate(-90deg); }
      #ring circle { fill: none; stroke-width: 3; }
      #ring .track { stroke: rgba(255, 255, 255, 0.18); }
      #arc { stroke: var(--accent); stroke-linecap: round; }
      #esc { position: absolute; bottom: 8px; left: 50%; transform: translateX(-50%); font: 500 11px Inter; white-space: nowrap; padding: 3px 9px; border-radius: 6px; background: rgba(40, 40, 46, 0.8); }
    </style>
  </head>
  <body>
    <div id="ring" class="glass"><svg viewBox="0 0 112 112"><circle class="track" cx="56" cy="56" r="54" /><circle id="arc" cx="56" cy="56" r="54" /></svg><span id="num"></span></div>
    <small id="esc">Esc or click to cancel</small>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/countdown/main.ts`:

```ts
import '../shared/glass.css';
import { invoke } from '@tauri-apps/api/core';

const secs = await invoke<number>('countdown_info');
const num = document.querySelector<HTMLElement>('#num')!;
const arc = document.querySelector<SVGCircleElement>('#arc')!;
const C = 2 * Math.PI * 54;
arc.style.strokeDasharray = `${C}`;
arc.style.strokeDashoffset = '0';
let left = secs;
num.textContent = String(left);
requestAnimationFrame(() => {
  arc.style.transition = `stroke-dashoffset ${secs}s linear`;
  arc.style.strokeDashoffset = `${C}`;
});
const timer = setInterval(() => {
  left -= 1;
  if (left > 0) num.textContent = String(left);
  else {
    clearInterval(timer);
    document.body.hidden = true;
    void invoke('countdown_done');
  }
}, 1000);
const cancel = () => {
  clearInterval(timer);
  void invoke('countdown_cancel');
};
addEventListener('keydown', (e) => e.key === 'Escape' && cancel());
addEventListener('mousedown', cancel);
```

In `vite.config.ts`, set `const pages = ['overlay', 'countdown'];`.

- [ ] **Step 6: Verify**

Run `npm run build && npm test`: no errors.

Then, with `npm run tauri dev` running:
- Open the overlay, click **Options**, and compare with mockup 2c. Check that the Save-to path, timer segments and three switches appear, and that toggles persist: close the overlay, reopen it, and look in `~/.config/rshot/config.toml`.
- Choose **Save to**: a folder dialog opens above the overlay.
- Set the timer to 3 s, select an area and press `⏎`:
  - The overlay disappears and a ring counts down 3 → 1 at the centre of the selection.
  - During the countdown you can open a menu in another app, and it appears in the saved PNG.
  - Try again and press `Esc` (or click the ring): nothing is saved.
- Set the timer back to Off.

- [ ] **Step 7: Commit**

```bash
git add -A src src-tauri vite.config.ts && git commit -m "feat: overlay options popover and timed captures on the live screen

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Floating thumbnail

**Files:**
- Create: `src-tauri/src/thumbnail.rs`, `src/thumbnail/index.html`, `src/thumbnail/thumbnail.css`, `src/thumbnail/main.ts`
- Modify: `src-tauri/src/pipeline.rs`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json`, `src/shared/ipc.ts`, `vite.config.ts`

**Interfaces:**
- Consumes: `ui::popup`, `AppState.{last_capture, clipboard, config}`.
- Scope note: in this plan, clicking the card opens the capture in the default image viewer, and there's no Edit button. Plan 2 adds the editor and sends the click and an Edit button there.
- Produces:
  - `thumbnail::Thumb { path, display, copied }` and `thumbnail::tildify(&Path) -> String`.
  - Commands: `thumbnail_info`, `read_capture(path)` (bytes), `reveal_capture(path)`, `open_capture(path)`, `delete_capture(path)`, `retry_copy(path)`, `dismiss_thumbnail`.
  - `ui::show_thumbnail(&AppHandle) -> Result<(), String>` and `AppState.thumb`.

- [ ] **Step 1: Add the plugins**

Run: `cargo add tauri-plugin-opener@2.5 tauri-plugin-drag@2.1`

In `main.rs`, add `.plugin(tauri_plugin_opener::init()).plugin(tauri_plugin_drag::init())`. In `capabilities/default.json`, add `"drag:default"`.

- [ ] **Step 2: Write `thumbnail.rs`**

```rust
//! Commands behind the floating thumbnail. Paths coming from the webview are checked first.

use crate::{err, store, ui, AppState};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{ipc::Response, AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize, Clone)]
pub struct Thumb {
    pub path: String,
    pub display: String,
    pub copied: bool,
}

/// `/home/me/Pictures/x.png` → `~/Pictures/x.png` (display only; the clipboard keeps the absolute path).
pub fn tildify(p: &Path) -> String {
    match dirs::home_dir().and_then(|h| p.strip_prefix(h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
    }
}

/// Only files rshot saved may be read, opened or deleted from a webview.
fn guard(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path).canonicalize().map_err(err)?;
    let dir = store::screenshots_dir(&state.config.lock().unwrap()).canonicalize().ok();
    let last = state.last_capture.lock().unwrap().as_ref().and_then(|l| l.canonicalize().ok());
    let inside = dir.is_some_and(|d| p.starts_with(d));
    if inside || last.as_deref() == Some(p.as_path()) {
        Ok(p)
    } else {
        Err("not an rshot capture".into())
    }
}

#[tauri::command]
pub fn thumbnail_info(state: State<'_, AppState>) -> Option<Thumb> {
    state.thumb.lock().unwrap().clone()
}

#[tauri::command]
pub async fn read_capture(app: AppHandle, path: String) -> Result<Response, String> {
    let state = app.state::<AppState>();
    let p = guard(&state, &path)?;
    std::fs::read(p).map(Response::new).map_err(err)
}

#[tauri::command]
pub fn reveal_capture(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    app.opener().reveal_item_in_dir(guard(&state, &path)?).map_err(err)
}

#[tauri::command]
pub fn open_capture(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = guard(&state, &path)?;
    ui::close_prefix(&app, "thumbnail");
    app.opener().open_path(p.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn delete_capture(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    std::fs::remove_file(guard(&state, &path)?).map_err(err)?;
    ui::close_prefix(&app, "thumbnail");
    Ok(())
}

#[tauri::command]
pub fn retry_copy(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = guard(&state, &path)?;
    let png = std::fs::read(&p).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&p, Some(&png), mode)?;
    if let Some(t) = state.thumb.lock().unwrap().as_mut() {
        t.copied = true;
    }
    Ok(())
}

#[tauri::command]
pub fn dismiss_thumbnail(app: AppHandle) {
    ui::close_prefix(&app, "thumbnail");
}
```

Add `show_thumbnail` to `ui.rs`:

```rust
/// Bottom-right of the monitor under the pointer.
pub fn show_thumbnail(app: &AppHandle) -> Result<(), String> {
    let p = app.cursor_position().map_err(err)?;
    let m = monitor_at(app, p.x, p.y)?;
    let s = m.scale_factor();
    let (w, h) = (270.0, 240.0);
    let win = popup(app, "thumbnail", "thumbnail/index.html", w, h, false)?;
    let x = m.position().x + m.size().width as i32 - ((w + 6.0) * s) as i32;
    let y = m.position().y + m.size().height as i32 - ((h + 6.0) * s) as i32;
    win.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    win.show().map_err(err)
}
```

In `pipeline::finish_capture`, replace everything from `let copied = …` to the end of the function with:

```rust
    let copied = state.clipboard.copy_capture(&path, Some(&png), cfg.clipboard_mode);
    if let Err(e) = &copied {
        eprintln!("rshot: clipboard: {e}");
    }
    *state.last_capture.lock().unwrap() = Some(path.clone());
    *state.thumb.lock().unwrap() = Some(crate::thumbnail::Thumb {
        path: path.display().to_string(),
        display: crate::thumbnail::tildify(&path),
        copied: copied.is_ok(),
    });
    if cfg.shutter_sound {
        play_shutter(app);
    }
    if cfg.show_thumbnail {
        // The capture already succeeded; a missing thumbnail must not turn it into an error.
        if let Err(e) = crate::ui::show_thumbnail(app) {
            eprintln!("rshot: thumbnail: {e}");
        }
    } else if copied.is_err() {
        notify(app, "Screenshot saved, but copying to the clipboard failed");
    }
    Ok(path)
}
```

In `main.rs`:
- Add `mod thumbnail;`.
- Add `pub thumb: std::sync::Mutex<Option<thumbnail::Thumb>>,` to `AppState` and `thumb: std::sync::Mutex::new(None),` to `new()`.
- Add `thumbnail::thumbnail_info, thumbnail::read_capture, thumbnail::reveal_capture, thumbnail::open_capture, thumbnail::delete_capture, thumbnail::retry_copy, thumbnail::dismiss_thumbnail,` to `generate_handler!`.

- [ ] **Step 3: Write the thumbnail page** (mockup 01-A, bottom)

Append to `src/shared/ipc.ts`:

```ts
export type Thumb = { path: string; display: string; copied: boolean };
export const thumbnailInfo = () => invoke<Thumb | null>('thumbnail_info');
export const readCapture = (path: string) => invoke<ArrayBuffer>('read_capture', { path });
export const revealCapture = (path: string) => invoke<void>('reveal_capture', { path });
export const openCapture = (path: string) => invoke<void>('open_capture', { path });
export const deleteCapture = (path: string) => invoke<void>('delete_capture', { path });
export const retryCopy = (path: string) => invoke<void>('retry_copy', { path });
export const dismissThumbnail = () => invoke<void>('dismiss_thumbnail');
```

`src/thumbnail/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot thumbnail</title>
  </head>
  <body>
    <div id="card" class="glass">
      <div class="shot">
        <img id="img" alt="Capture preview" draggable="false" />
        <div class="acts">
          <button data-act="reveal" data-icon="i-folder" title="Show in folder" aria-label="Show in folder"></button>
          <button data-act="delete" data-icon="i-trash" title="Delete" aria-label="Delete"></button>
        </div>
      </div>
      <div class="foot">
        <span class="ok" id="ok"><svg class="ic" aria-hidden="true"><use href="#i-check" /></svg></span>
        <div class="txt"><b id="status">Path copied</b><small id="path"></small></div>
        <button id="retry" class="b2" hidden>Retry</button>
      </div>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/thumbnail/thumbnail.css`:

```css
html, body { width: 100%; height: 100%; overflow: hidden; }
#card { position: absolute; right: 12px; bottom: 12px; width: 230px; border-radius: 12px; overflow: hidden; background: rgba(40, 40, 46, 0.7); transition: transform 0.2s ease, opacity 0.2s ease; animation: in 0.22s ease-out; }
@keyframes in { from { transform: translateY(12px); opacity: 0; } }
#card.out { transform: translateX(260px); opacity: 0; }
.shot { position: relative; height: 132px; background: #111; display: grid; place-items: center; }
.shot img { max-width: 100%; max-height: 100%; object-fit: contain; }
.acts { position: absolute; top: 8px; right: 8px; display: flex; gap: 4px; opacity: 0; transition: opacity 0.15s; }
#card:hover .acts { opacity: 1; }
.acts button { width: 26px; height: 26px; border-radius: 7px; display: grid; place-items: center; background: rgba(20, 20, 24, 0.72); color: #fff; }
.acts .ic { width: 13px; height: 13px; }
.foot { display: flex; align-items: center; gap: 7px; padding: 8px 10px; font: 500 11px Inter; }
.ok { width: 16px; height: 16px; border-radius: 50%; background: var(--ok); display: grid; place-items: center; color: #062; flex-shrink: 0; }
.ok .ic { width: 10px; height: 10px; stroke-width: 3; }
.ok.fail { background: var(--danger); color: #fff; }
.txt { min-width: 0; flex: 1; }
.txt small { display: block; color: rgba(255, 255, 255, 0.55); font-size: 10px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
#retry { height: 24px; padding: 0 9px; font-size: 11px; }
```

`src/thumbnail/main.ts`:

```ts
import '../shared/glass.css';
import './thumbnail.css';
import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const card = document.querySelector<HTMLElement>('#card')!;
const img = document.querySelector<HTMLImageElement>('#img')!;
const DISMISS_MS = 5000;

function setCopied(ok: boolean) {
  document.querySelector('#status')!.textContent = ok ? 'Path copied' : 'Copy failed';
  document.querySelector('#ok')!.classList.toggle('fail', !ok);
  document.querySelector<HTMLElement>('#retry')!.hidden = ok;
}

/** Small PNG for the drag cursor (the full capture would be enormous). */
function dragIcon(): string {
  const w = 160;
  const h = Math.max(1, Math.round((img.naturalHeight / Math.max(1, img.naturalWidth)) * w));
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  c.getContext('2d')!.drawImage(img, 0, 0, w, h);
  return c.toDataURL('image/png');
}

async function run() {
  const t = await ipc.thumbnailInfo();
  if (!t) return void ipc.dismissThumbnail();
  img.src = URL.createObjectURL(new Blob([await ipc.readCapture(t.path)], { type: 'image/png' }));
  document.querySelector('#path')!.textContent = t.display;
  setCopied(t.copied);

  let left = DISMISS_MS;
  let last = performance.now();
  let hovering = false;
  let gone = false;
  const dismiss = () => {
    if (gone) return;
    gone = true;
    card.classList.add('out');
    setTimeout(() => void ipc.dismissThumbnail(), 200);
  };
  const tick = (now: number) => {
    if (!hovering) left -= now - last;
    last = now;
    if (left <= 0) dismiss();
    else if (!gone) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  card.addEventListener('mouseenter', () => (hovering = true));
  card.addEventListener('mouseleave', () => (hovering = false));

  card.addEventListener('click', async (e) => {
    const b = (e.target as Element).closest<HTMLButtonElement>('button');
    if (!b) return;
    if (b.dataset.act === 'reveal') await ipc.revealCapture(t.path);
    else if (b.dataset.act === 'delete') await ipc.deleteCapture(t.path);
    else if (b.id === 'retry') {
      await ipc.retryCopy(t.path).then(() => setCopied(true)).catch(() => setCopied(false));
    }
  });

  // Click → open; drag right → swipe away; any other drag → drag the file into another app.
  card.addEventListener('pointerdown', (e) => {
    if ((e.target as Element).closest('button')) return;
    const sx = e.clientX;
    const sy = e.clientY;
    let mode: 'none' | 'swipe' | 'file' = 'none';
    const onMove = (m: PointerEvent) => {
      const dx = m.clientX - sx;
      const dy = m.clientY - sy;
      if (mode === 'none' && Math.hypot(dx, dy) > 6) {
        mode = dx > 0 && Math.abs(dx) > Math.abs(dy) ? 'swipe' : 'file';
        if (mode === 'file') {
          cleanup();
          void startDrag({ item: [t.path], icon: dragIcon() }).catch(() => {});
          return;
        }
      }
      if (mode === 'swipe') card.style.transform = `translateX(${Math.max(0, dx)}px)`;
    };
    const onUp = (u: PointerEvent) => {
      cleanup();
      if (mode === 'swipe') {
        if (u.clientX - sx > 80) dismiss();
        else card.style.transform = '';
      } else if (mode === 'none') void ipc.openCapture(t.path);
    };
    const cleanup = () => {
      removeEventListener('pointermove', onMove);
      removeEventListener('pointerup', onUp);
    };
    addEventListener('pointermove', onMove);
    addEventListener('pointerup', onUp);
  });
}

void run();
```

In `vite.config.ts`, set `const pages = ['overlay', 'countdown', 'thumbnail'];`.

- [ ] **Step 4: Verify**

Run `npm run build && npm test`: no errors.

Then, with dev running, capture an area and compare with mockup 01-A (bottom):
- A glass card slides in at the bottom-right of the monitor under the pointer, showing the image, "✓ Path copied" and `~/Pictures/Screenshots/Screenshot_….png`.
- It stays 5 s and fades. Hovering pauses the timer and shows the folder and trash buttons.
- **Folder** opens Files with the capture selected. **Trash** deletes the file and closes the card.
- Clicking the card opens the image in the default viewer.
- Dragging right past 80 px swipes the card away.
- Dragging up or left starts a file drag. Drop it into the Files window or a Chrome upload box: the file is copied or attached.
- In `config.toml`, set `show_thumbnail = false` and restart dev: a capture shows no card, but the sound plays and the clipboard is set. Set it back to true.

- [ ] **Step 5: Commit**

```bash
git add -A src src-tauri vite.config.ts && git commit -m "feat: floating thumbnail with open, reveal, delete, swipe and drag-out

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: GNOME shortcut takeover and restore

**Files:**
- Create: `src-tauri/src/shortcuts/mod.rs`, `src-tauri/src/shortcuts/gvariant.rs`, `src-tauri/src/shortcuts/gnome.rs`
- Modify: `src-tauri/src/main.rs`
- Test: unit tests inside `gvariant.rs`

**Interfaces:**
- Consumes: `store::{Config, load_config, save_config}`.
- Produces:
  - `shortcuts::take_over(&mut Config) -> Result<(), String>` (it sets `gnome_backup`; the caller sets `takeover`).
  - `shortcuts::restore(&mut Config) -> Result<(), String>` (it clears `gnome_backup`).
  - `shortcuts::restore_from_cli() -> Result<(), String>` and `shortcuts::manual_commands(&Config) -> Vec<(String, String)>`.
  - Linux only, in `gvariant`: `quote`, `parse_strv`, `format_strv`, `to_gnome_accel`, `with_paths` and `without_paths`.

- [ ] **Step 1: Write the failing GVariant tests**

`src-tauri/src/shortcuts/gvariant.rs`:

```rust
//! The little GVariant text that `gsettings get/set` speaks, plus accelerator conversion.

pub fn quote(_s: &str) -> String {
    String::new()
}

pub fn parse_strv(_s: &str) -> Vec<String> {
    Vec::new()
}

pub fn format_strv(_v: &[String]) -> String {
    String::new()
}

/// "Ctrl+Alt+Shift+R" → "<Ctrl><Alt><Shift>R" (GTK accelerator syntax).
pub fn to_gnome_accel(_neutral: &str) -> String {
    String::new()
}

/// Adds `ours` to a custom-keybindings list without duplicates, keeping everything else.
pub fn with_paths(existing: &[String], _ours: &[String]) -> Vec<String> {
    existing.to_vec()
}

pub fn without_paths(existing: &[String], _ours: &[String]) -> Vec<String> {
    existing.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn quotes_and_escapes() {
        assert_eq!(quote("Print"), "'Print'");
        assert_eq!(quote("it's \\ ok"), r"'it\'s \\ ok'");
    }

    #[test]
    fn parses_gsettings_output() {
        assert_eq!(parse_strv("['Print']"), v(&["Print"]));
        assert_eq!(parse_strv("@as []"), Vec::<String>::new());
        assert_eq!(parse_strv("['<Shift>Print', 'a\\'b']"), v(&["<Shift>Print", "a'b"]));
        assert_eq!(parse_strv("[\"x\"]"), v(&["x"]));
    }

    #[test]
    fn formats_for_gsettings() {
        assert_eq!(format_strv(&[]), "@as []");
        assert_eq!(format_strv(&v(&["/a/", "/b/"])), "['/a/', '/b/']");
        assert_eq!(parse_strv(&format_strv(&v(&["x'y"]))), v(&["x'y"]));
    }

    #[test]
    fn converts_accelerators() {
        assert_eq!(to_gnome_accel("Print"), "Print");
        assert_eq!(to_gnome_accel("Shift+Print"), "<Shift>Print");
        assert_eq!(to_gnome_accel("Ctrl+Alt+Shift+R"), "<Ctrl><Alt><Shift>R");
        assert_eq!(to_gnome_accel("Super+4"), "<Super>4");
    }

    #[test]
    fn merges_and_removes_custom_paths() {
        let ours = v(&["/r/a/", "/r/b/"]);
        assert_eq!(with_paths(&v(&["/x/", "/r/a/"]), &ours), v(&["/x/", "/r/a/", "/r/b/"]));
        assert_eq!(without_paths(&v(&["/x/", "/r/a/", "/r/b/"]), &ours), v(&["/x/"]));
    }
}
```

`src-tauri/src/shortcuts/mod.rs` (the first version: just the module):

```rust
//! Taking over / giving back the OS screenshot shortcuts.

#[cfg(target_os = "linux")]
pub mod gvariant;
```

In `main.rs`, add `mod shortcuts;`.

Run: `cargo test gvariant`
Expected: all 5 tests FAIL.

- [ ] **Step 2: Implement `gvariant.rs`**

Replace the stub functions:

```rust
pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Reads every quoted string in a GVariant string array ("['a', 'b']", "@as []").
pub fn parse_strv(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\'' && c != '"' {
            continue;
        }
        let mut cur = String::new();
        while let Some(d) = chars.next() {
            match d {
                '\\' => cur.extend(chars.next()),
                _ if d == c => break,
                _ => cur.push(d),
            }
        }
        out.push(cur);
    }
    out
}

pub fn format_strv(v: &[String]) -> String {
    if v.is_empty() {
        "@as []".into()
    } else {
        format!("[{}]", v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(", "))
    }
}

pub fn to_gnome_accel(neutral: &str) -> String {
    let parts: Vec<&str> = neutral.split('+').collect();
    let (key, mods) = parts.split_last().expect("split always yields one part");
    let mods: String = mods
        .iter()
        .map(|m| match *m {
            "Ctrl" => "<Ctrl>",
            "Alt" => "<Alt>",
            "Shift" => "<Shift>",
            _ => "<Super>",
        })
        .collect();
    format!("{mods}{key}")
}

pub fn with_paths(existing: &[String], ours: &[String]) -> Vec<String> {
    let mut v = existing.to_vec();
    v.extend(ours.iter().filter(|p| !existing.contains(p)).cloned());
    v
}

pub fn without_paths(existing: &[String], ours: &[String]) -> Vec<String> {
    existing.iter().filter(|p| !ours.contains(p)).cloned().collect()
}
```

Run: `cargo test gvariant`
Expected: `5 passed`.

- [ ] **Step 3: Write `gnome.rs` (the gsettings I/O)**

```rust
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
```

- [ ] **Step 4: Complete `shortcuts/mod.rs`**

```rust
//! Taking over / giving back the OS screenshot shortcuts.

#[cfg(target_os = "linux")]
mod gnome;
#[cfg(target_os = "linux")]
pub mod gvariant;

use crate::store::{self, Config};

/// The binary the bindings should run (the AppImage when running from one).
fn exe_command() -> String {
    std::env::var("APPIMAGE")
        .ok()
        .or_else(|| std::env::current_exe().ok().map(|p| p.display().to_string()))
        .unwrap_or_else(|| "rshot".into())
}

#[cfg(target_os = "linux")]
pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    gnome::take_over(cfg, &exe_command())
}

#[cfg(target_os = "linux")]
pub fn restore(cfg: &mut Config) -> Result<(), String> {
    gnome::restore(cfg)
}

#[cfg(not(target_os = "linux"))]
pub fn take_over(_cfg: &mut Config) -> Result<(), String> {
    Err("Taking over the system shortcuts on this OS arrives in a later version.".into())
}

#[cfg(not(target_os = "linux"))]
pub fn restore(_cfg: &mut Config) -> Result<(), String> {
    Ok(())
}

/// `rshot restore-shortcuts`: works with no daemon running (used by uninstallers).
pub fn restore_from_cli() -> Result<(), String> {
    let mut c = store::load_config();
    restore(&mut c)?;
    c.takeover = false;
    store::save_config(&c).map_err(|e| e.to_string())
}

/// (shortcut, command) pairs to bind by hand on desktops rshot can't configure.
pub fn manual_commands(c: &Config) -> Vec<(String, String)> {
    let exe = exe_command();
    vec![
        (c.shortcuts.area.clone(), format!("\"{exe}\" capture area")),
        (c.shortcuts.screen.clone(), format!("\"{exe}\" capture screen")),
        (c.shortcuts.window.clone(), format!("\"{exe}\" capture window")),
    ]
}
```

In `main.rs`, handle `restore-shortcuts` before Tauri starts. Insert this directly after `let cmd = …;`:

```rust
    if cmd == cli::Cmd::RestoreShortcuts {
        std::process::exit(match shortcuts::restore_from_cli() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("rshot: {e}");
                1
            }
        });
    }
```

- [ ] **Step 5: Verify take-over and restore on the real desktop**

Temporarily add a test hook in `setup`, and **remove it at the end of this step**:

```rust
            if std::env::var("RSHOT_TAKEOVER_TEST").is_ok() {
                let state = app.state::<AppState>();
                let mut c = state.config.lock().unwrap();
                shortcuts::take_over(&mut c).expect("take over");
                c.takeover = true;
                store::save_config(&c).unwrap();
            }
```

Add `use tauri::Manager;` at the top of `main.rs` if the compiler asks for it. Then:

```bash
for k in show-screenshot-ui screenshot screenshot-window; do gsettings get org.gnome.shell.keybindings $k; done > /tmp/before.txt
gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings >> /tmp/before.txt
RSHOT_TAKEOVER_TEST=1 npm run tauri dev > /tmp/rshot-dev.log 2>&1 &    # wait for "dispatch Daemon"
gsettings get org.gnome.shell.keybindings show-screenshot-ui      # expect: @as []
gsettings get org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rshot-area/ command
xdotool key Print; sleep 1.5; xdotool search --onlyvisible --name 'rshot overlay' | wc -l   # expect ≥ 1
xdotool key Escape
xdotool key shift+Print; sleep 1; ls -t ~/Pictures/Screenshots | head -1                   # new file
pkill -f 'target/debug/rshot'; ./src-tauri/target/debug/rshot restore-shortcuts; echo "exit=$?"
for k in show-screenshot-ui screenshot screenshot-window; do gsettings get org.gnome.shell.keybindings $k; done > /tmp/after.txt
gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings >> /tmp/after.txt
diff /tmp/before.txt /tmp/after.txt && echo RESTORED
```

Expected:
- `show-screenshot-ui` reads `@as []` while rshot owns the keys.
- The rshot-area command is `'"/…/target/debug/rshot" capture area'`.
- `Print` opens the rshot overlay and `Shift+Print` saves a file.
- `restore-shortcuts` exits `0`, and the diff is empty (`RESTORED`).

Now **delete the `RSHOT_TAKEOVER_TEST` block** from `setup`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri && git commit -m "feat: GNOME shortcut takeover with backup and restore-shortcuts CLI

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Onboarding, Settings, final tray menu

**Files:**
- Create: `src-tauri/src/settings.rs`, `src/settings/{index.html,settings.css,main.ts,keys.ts,keys.test.ts}`, `src/onboarding/{index.html,main.ts}`
- Modify: `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src/shared/ipc.ts`, `vite.config.ts`

**Interfaces:**
- Consumes: `shortcuts::{take_over, restore, manual_commands}`, `store::{dir_setting, screenshots_dir, config_path}`, `pick_folder` (Task 8).
- Produces:
  - `settings::Settings` (the JSON is shown in the `ipc.ts` types below).
  - Commands: `get_settings`, `set_settings(settings) -> Settings`, `onboarding_choice(accept)`, `open_config`, `close_window`.
  - `ui::{open_settings, open_onboarding}`, the final tray menu, and `keys.ts::comboFrom`.

- [ ] **Step 1: Write the failing key-combo test**

`src/settings/keys.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { comboFrom } from './keys';

const ev = (code: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe('comboFrom', () => {
  it('builds modifier combos in a fixed order', () => {
    expect(comboFrom(ev('KeyR', { ctrlKey: true, altKey: true, shiftKey: true }))).toBe('Ctrl+Alt+Shift+R');
    expect(comboFrom(ev('Digit4', { metaKey: true, shiftKey: true }))).toBe('Shift+Super+4');
  });
  it('accepts Print and F-keys alone', () => {
    expect(comboFrom(ev('PrintScreen'))).toBe('Print');
    expect(comboFrom(ev('F12'))).toBe('F12');
  });
  it('rejects bare letters and lone modifiers', () => {
    expect(comboFrom(ev('KeyF'))).toBeNull();
    expect(comboFrom(ev('ShiftLeft', { shiftKey: true }))).toBeNull();
  });
});
```

`src/settings/keys.ts` (stub):

```ts
type KeyLike = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

export function comboFrom(_e: KeyLike): string | null {
  return null;
}
```

Run: `npm test`
Expected: the `comboFrom` tests FAIL (the first two cases get `null`).

- [ ] **Step 2: Implement `keys.ts`**

```ts
type KeyLike = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** KeyboardEvent → neutral "Ctrl+Alt+Shift+R" combo, or null if it can't be a shortcut. */
export function comboFrom(e: KeyLike): string | null {
  const c = e.code;
  const key =
    c === 'PrintScreen' ? 'Print' : c.startsWith('Key') ? c.slice(3) : c.startsWith('Digit') ? c.slice(5) : /^F\d{1,2}$/.test(c) ? c : null;
  if (!key) return null;
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(
    (m): m is string => !!m,
  );
  if (!mods.length && key !== 'Print' && !/^F\d{1,2}$/.test(key)) return null;
  return [...mods, key].join('+');
}
```

Run: `npm test`
Expected: all tests pass.

- [ ] **Step 3: Write `settings.rs`**

Run: `cargo add tauri-plugin-autostart@2.5`. In `main.rs`, add `.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))`.

```rust
//! Settings window and first-run dialog.

use crate::{
    err, shortcuts,
    store::{self, ClipboardMode, Shortcuts},
    AppState,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub launch_at_login: bool,
    pub screenshots_dir: String,
    pub clipboard_mode: ClipboardMode,
    pub show_thumbnail: bool,
    pub shutter_sound: bool,
    pub takeover: bool,
    pub shortcuts: Shortcuts,
    #[serde(default)]
    pub takeover_error: Option<String>,
    #[serde(default)]
    pub manual: Vec<(String, String)>,
}

fn snapshot(app: &AppHandle, c: &store::Config, takeover_error: Option<String>) -> Settings {
    Settings {
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        screenshots_dir: store::screenshots_dir(c).display().to_string(),
        clipboard_mode: c.clipboard_mode,
        show_thumbnail: c.show_thumbnail,
        shutter_sound: c.shutter_sound,
        takeover: c.takeover,
        shortcuts: c.shortcuts.clone(),
        takeover_error,
        manual: shortcuts::manual_commands(c),
    }
}

/// Takes the shortcuts over; on failure, puts back whatever was already changed.
fn take_over_or_roll_back(c: &mut store::Config) -> Result<(), String> {
    match shortcuts::take_over(c) {
        Ok(()) => {
            c.takeover = true;
            Ok(())
        }
        Err(e) => {
            let _ = shortcuts::restore(c);
            c.takeover = false;
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Settings {
    snapshot(&app, &state.config.lock().unwrap(), None)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Settings {
    let mut c = state.config.lock().unwrap();
    let mut error = None;
    let autostart = app.autolaunch();
    let was = autostart.is_enabled().unwrap_or(false);
    if settings.launch_at_login != was {
        let r = if settings.launch_at_login { autostart.enable() } else { autostart.disable() };
        if let Err(e) = r {
            error = Some(format!("Launch at login: {e}"));
        }
    }
    c.screenshots_dir = store::dir_setting(&settings.screenshots_dir);
    c.clipboard_mode = settings.clipboard_mode;
    c.show_thumbnail = settings.show_thumbnail;
    c.shutter_sound = settings.shutter_sound;
    let rebound = c.shortcuts != settings.shortcuts;
    c.shortcuts = settings.shortcuts;
    if settings.takeover && (!c.takeover || rebound) {
        if let Err(e) = take_over_or_roll_back(&mut c) {
            error = Some(e);
        }
    } else if !settings.takeover && c.takeover {
        match shortcuts::restore(&mut c) {
            Ok(()) => c.takeover = false,
            Err(e) => error = Some(e),
        }
    }
    if let Err(e) = store::save_config(&c) {
        error = Some(format!("Saving settings: {e}"));
    }
    snapshot(&app, &c, error)
}

#[tauri::command]
pub fn onboarding_choice(app: AppHandle, state: State<'_, AppState>, accept: bool) -> Result<(), String> {
    let mut c = state.config.lock().unwrap();
    c.onboarded = true;
    let result = if accept {
        let _ = app.autolaunch().enable();
        take_over_or_roll_back(&mut c)
    } else {
        Ok(())
    };
    store::save_config(&c).map_err(err)?;
    result
}

#[tauri::command]
pub fn open_config(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    store::save_config(&state.config.lock().unwrap()).map_err(err)?;
    app.opener().open_path(store::config_path().to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn close_window(window: WebviewWindow) {
    let _ = window.destroy();
}
```

- [ ] **Step 4: Add the windows and the final tray menu to `ui.rs`**

```rust
fn dialog_window(app: &AppHandle, label: &str, page: &str, title: &str, w: f64, h: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(label) {
        win.show().map_err(err)?;
        return win.set_focus().map_err(err);
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .inner_size(w, h)
        .center()
        .build()
        .map(|_| ())
        .map_err(err)
}

pub fn open_settings(app: &AppHandle) -> Result<(), String> {
    dialog_window(app, "settings", "settings/index.html", "rshot Settings", 720.0, 780.0)
}

pub fn open_onboarding(app: &AppHandle) -> Result<(), String> {
    dialog_window(app, "onboarding", "onboarding/index.html", "Welcome to rshot", 480.0, 470.0)
}
```

Replace `create_tray` with the menu from mockup 05a (without Record, which comes in Plan 3):

```rust
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let sep = || tauri::menu::PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[
            &item("area", "Capture Area")?,
            &item("screen", "Capture Screen")?,
            &item("window", "Capture Window")?,
            &sep()?,
            &item("last", "Open Last Capture")?,
            &item("folder", "Open Screenshots Folder")?,
            &sep()?,
            &item("settings", "Settings…")?,
            &item("quit", "Quit rshot")?,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("bundle icon is configured"))
        .tooltip("rshot")
        .menu(&menu)
        .on_menu_event(|app, e| {
            let result = match e.id.as_ref() {
                "quit" => return app.exit(0),
                "settings" => open_settings(app),
                "last" => open_last(app),
                "folder" => open_folder(app),
                id => {
                    let cmd = match id {
                        "area" => Cmd::CaptureArea,
                        "screen" => Cmd::CaptureScreen,
                        "window" => Cmd::CaptureWindow,
                        _ => return,
                    };
                    let app = app.clone();
                    // Let the tray menu close so it isn't in the capture.
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(300));
                        crate::dispatch(&app, cmd);
                    });
                    Ok(())
                }
            };
            if let Err(e) = result {
                crate::pipeline::notify(app, &e);
            }
        })
        .build(app)?;
    Ok(())
}

fn open_last(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let last = app.state::<crate::AppState>().last_capture.lock().unwrap().clone();
    let path = last.ok_or("No capture yet")?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(err)
}

fn open_folder(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::store::screenshots_dir(&app.state::<crate::AppState>().config.lock().unwrap());
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(err)
}
```

In `main.rs`:
- Add `mod settings;`.
- Add `settings::get_settings, settings::set_settings, settings::onboarding_choice, settings::open_config, settings::close_window,` to `generate_handler!`.
- In `setup`, after `ensure_overlays`, add:

  ```rust
  if !app.state::<AppState>().config.lock().unwrap().onboarded {
      ui::open_onboarding(app.handle())?;
  }
  ```

  (This needs `use tauri::Manager;`.)

Append to `src/shared/ipc.ts`:

```ts
export type ClipboardMode = 'path-and-image' | 'path-only';
export type Shortcuts = { area: string; screen: string; window: string };
export type Settings = {
  launch_at_login: boolean;
  screenshots_dir: string;
  clipboard_mode: ClipboardMode;
  show_thumbnail: boolean;
  shutter_sound: boolean;
  takeover: boolean;
  shortcuts: Shortcuts;
  takeover_error: string | null;
  manual: [string, string][];
};
export type BoolSetting = 'launch_at_login' | 'show_thumbnail' | 'shutter_sound' | 'takeover';
export const getSettings = () => invoke<Settings>('get_settings');
export const setSettings = (settings: Settings) => invoke<Settings>('set_settings', { settings });
export const onboardingChoice = (accept: boolean) => invoke<void>('onboarding_choice', { accept });
export const openConfig = () => invoke<void>('open_config');
export const closeWindow = () => invoke<void>('close_window');
```

- [ ] **Step 5: Write the Settings page** (mockup 05c; the Recording group comes in Plan 3)

`src/settings/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot Settings</title>
  </head>
  <body>
    <div class="win">
      <header data-tauri-drag-region>rshot Settings<button class="wx" id="close" data-icon="i-x" aria-label="Close"></button></header>
      <main>
        <section>
          <h5>General</h5>
          <div class="box">
            <div class="row"><span class="l">Launch at login</span><button class="switch" role="switch" data-key="launch_at_login" aria-label="Launch at login"></button></div>
            <div class="row"><span class="l">Screenshots folder</span><button class="fld" id="folder"><svg class="ic" aria-hidden="true"><use href="#i-folder" /></svg><span></span></button></div>
          </div>
        </section>
        <section>
          <h5>After capture</h5>
          <div class="box">
            <div class="row">
              <span class="l">Clipboard<small>Path + image: terminals get the path, chat apps get the image</small></span>
              <div class="seg" id="clip"><button data-v="path-and-image">Path + image</button><button data-v="path-only">Path only</button></div>
            </div>
            <div class="row"><span class="l">Show floating thumbnail</span><button class="switch" role="switch" data-key="show_thumbnail" aria-label="Show floating thumbnail"></button></div>
            <div class="row"><span class="l">Shutter sound</span><button class="switch" role="switch" data-key="shutter_sound" aria-label="Shutter sound"></button></div>
          </div>
        </section>
        <section>
          <h5>Shortcuts</h5>
          <div class="box">
            <div class="row">
              <span class="l">Take over system screenshot shortcuts<small>Turning this off gives the shortcuts back to GNOME Screenshot</small></span>
              <button class="switch" role="switch" data-key="takeover" aria-label="Take over system screenshot shortcuts"></button>
            </div>
            <div class="row err" id="err" hidden></div>
            <div class="row"><span class="l">Capture area</span><button class="keys" data-shortcut="area"></button></div>
            <div class="row"><span class="l">Capture screen</span><button class="keys" data-shortcut="screen"></button></div>
            <div class="row"><span class="l">Capture window</span><button class="keys" data-shortcut="window"></button></div>
          </div>
        </section>
        <footer><span id="version"></span><button class="link" id="config">Open config file</button></footer>
      </main>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/settings/settings.css`:

```css
html, body { height: 100%; overflow: hidden; }
.win { height: 100%; display: flex; flex-direction: column; border-radius: 12px; overflow: hidden; background: var(--surface); border: 1px solid #34343a; }
header { height: 46px; flex-shrink: 0; background: var(--surface-2); border-bottom: 1px solid var(--line); display: flex; align-items: center; justify-content: center; position: relative; font: 600 13px Inter; }
.wx { position: absolute; right: 12px; width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.1); }
.wx .ic { width: 12px; height: 12px; }
main { flex: 1; overflow-y: auto; padding: 18px 26px 24px; }
section { margin-bottom: 18px; }
h5 { font: 600 12px Inter; color: var(--text-2); margin: 0 0 7px 4px; }
.box { background: rgba(255, 255, 255, 0.045); border: 1px solid var(--line); border-radius: 12px; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 10px 14px; border-top: 1px solid rgba(255, 255, 255, 0.06); min-height: 46px; }
.row:first-child { border-top: 0; }
.l { font: 500 13px Inter; }
.l small { display: block; font: 400 11.5px Inter; color: rgba(255, 255, 255, 0.5); margin-top: 1px; }
.switch { width: 36px; height: 22px; border-radius: 11px; }
.switch::after { width: 18px; height: 18px; }
.switch.on::after { left: 16px; }
.fld { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 10px; border-radius: 8px; background: var(--fill); font: 500 12px Inter; color: rgba(255, 255, 255, 0.85); max-width: 380px; }
.fld span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.fld .ic { width: 14px; height: 14px; color: var(--text-2); }
.seg button { padding: 5px 12px; }
.keys { display: flex; gap: 4px; align-items: center; min-height: 30px; }
.keys kbd { font-size: 10.5px; border: 1px solid rgba(255, 255, 255, 0.1); border-bottom-width: 2px; border-radius: 6px; padding: 2px 7px; background: rgba(255, 255, 255, 0.12); }
.keys.rebind { height: 30px; padding: 0 12px; border-radius: 8px; font: 500 12px Inter; color: var(--accent); box-shadow: inset 0 0 0 1.5px var(--accent); background: rgba(10, 132, 255, 0.1); }
.err { display: block; font: 400 11.5px Inter; color: rgba(255, 255, 255, 0.7); line-height: 1.5; }
.err p { margin-bottom: 6px; }
.err p:first-child { color: #ff8a80; }
.err div { display: flex; gap: 10px; align-items: center; margin-top: 4px; }
.err code { font: 500 11px ui-monospace, monospace; background: rgba(0, 0, 0, 0.3); padding: 2px 6px; border-radius: 5px; user-select: text; -webkit-user-select: text; }
footer { display: flex; justify-content: space-between; font: 500 11.5px Inter; color: var(--text-3); padding: 2px 4px; }
.link { color: var(--accent); font: 500 12px Inter; }
```

`src/settings/main.ts`:

```ts
import '../shared/glass.css';
import './settings.css';
import { getVersion } from '@tauri-apps/api/app';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import { comboFrom } from './keys';

mountIcons();
let s = await ipc.getSettings();
let rebinding: HTMLElement | null = null;

const esc = (t: string) => t.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);

function render() {
  document.querySelectorAll<HTMLElement>('[data-key]').forEach((b) => {
    const on = s[b.dataset.key as ipc.BoolSetting];
    b.classList.toggle('on', on);
    b.setAttribute('aria-checked', String(on));
  });
  document.querySelector('#folder span')!.textContent = s.screenshots_dir;
  document.querySelectorAll<HTMLElement>('#clip button').forEach((b) => b.classList.toggle('on', b.dataset.v === s.clipboard_mode));
  document.querySelectorAll<HTMLElement>('[data-shortcut]').forEach((b) => {
    if (b === rebinding) {
      b.className = 'keys rebind';
      b.textContent = 'Press new shortcut…  Esc to cancel';
      return;
    }
    b.className = 'keys';
    b.innerHTML = s.shortcuts[b.dataset.shortcut as keyof ipc.Shortcuts]
      .split('+')
      .map((k) => `<kbd>${esc(k)}</kbd>`)
      .join('');
  });
  const err = document.querySelector<HTMLElement>('#err')!;
  err.hidden = !s.takeover_error;
  if (s.takeover_error) {
    err.innerHTML =
      `<p>${esc(s.takeover_error)}</p><p>Bind these yourself in your desktop's keyboard settings:</p>` +
      s.manual.map(([k, c]) => `<div><kbd>${esc(k)}</kbd><code>${esc(c)}</code></div>`).join('');
  }
}

async function save() {
  s = await ipc.setSettings(s);
  render();
}

document.addEventListener('click', async (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.key) {
    const k = b.dataset.key as ipc.BoolSetting;
    s[k] = !s[k];
    await save();
  } else if (b.dataset.v) {
    s.clipboard_mode = b.dataset.v as ipc.ClipboardMode;
    await save();
  } else if (b.id === 'folder') {
    const d = await ipc.pickFolder();
    if (d) {
      s.screenshots_dir = d;
      await save();
    }
  } else if (b.dataset.shortcut) {
    rebinding = b;
    render();
  } else if (b.id === 'config') await ipc.openConfig();
  else if (b.id === 'close') await ipc.closeWindow();
});

addEventListener('keydown', async (e) => {
  if (!rebinding) return;
  e.preventDefault();
  if (e.key === 'Escape') {
    rebinding = null;
    return render();
  }
  const combo = comboFrom(e);
  if (!combo) return;
  s.shortcuts[rebinding.dataset.shortcut as keyof ipc.Shortcuts] = combo;
  rebinding = null;
  await save();
});

document.querySelector('#version')!.textContent = `rshot ${await getVersion()}`;
render();
```

- [ ] **Step 6: Write the onboarding page** (mockup 05b)

`src/onboarding/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Welcome to rshot</title>
    <style>
      html, body { height: 100%; overflow: hidden; }
      .dlg { height: 100%; background: var(--surface-2); border: 1px solid #38383f; border-radius: 16px; padding: 22px; display: flex; flex-direction: column; }
      .logo { width: 42px; height: 42px; border-radius: 11px; background: linear-gradient(135deg, #0a84ff, #5e5ce6); display: grid; place-items: center; margin-bottom: 12px; box-shadow: 0 6px 16px rgba(10, 132, 255, 0.35); color: #fff; }
      .logo .ic { width: 24px; height: 24px; }
      h4 { font: 700 16px Inter; margin-bottom: 4px; }
      p { font: 400 12.5px Inter; color: rgba(255, 255, 255, 0.65); line-height: 1.5; margin-bottom: 14px; }
      .klist { background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.07); border-radius: 10px; margin-bottom: 16px; }
      .klist div { display: flex; justify-content: space-between; align-items: center; padding: 7px 12px; font: 500 12px Inter; border-top: 1px solid rgba(255, 255, 255, 0.06); }
      .klist div:first-child { border-top: 0; }
      kbd { font-size: 10.5px; border: 1px solid rgba(255, 255, 255, 0.1); border-bottom-width: 2px; border-radius: 6px; padding: 2px 7px; background: rgba(255, 255, 255, 0.12); }
      #err { color: #ff8a80; font: 400 12px Inter; margin-bottom: 10px; }
      #err code { font: 500 11px ui-monospace, monospace; color: #fff; display: block; margin-top: 4px; user-select: text; -webkit-user-select: text; }
      .acts { margin-top: auto; display: flex; justify-content: flex-end; gap: 8px; }
      .b1, .b2 { height: 32px; font-size: 12.5px; }
    </style>
  </head>
  <body>
    <div class="dlg" data-tauri-drag-region>
      <div class="logo" data-icon="i-logo"></div>
      <h4>Make rshot your screenshot tool?</h4>
      <p>rshot will take over these shortcuts from GNOME Screenshot and start when you log in. You can give the shortcuts back at any time in Settings, and uninstalling rshot restores them too.</p>
      <div class="klist" id="keys"></div>
      <div id="err" hidden></div>
      <div class="acts"><button class="b2" id="no">Not now</button><button class="b1" id="yes">Use rshot</button></div>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/onboarding/main.ts`:

```ts
import '../shared/glass.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const s = await ipc.getSettings();
const rows: [string, string][] = [
  ['Capture area (with toolbar)', s.shortcuts.area],
  ['Capture screen', s.shortcuts.screen],
  ['Capture window', s.shortcuts.window],
];
document.querySelector('#keys')!.innerHTML = rows.map(([label, key]) => `<div>${label}<kbd>${key}</kbd></div>`).join('');

document.querySelector('#no')!.addEventListener('click', async () => {
  await ipc.onboardingChoice(false);
  await ipc.closeWindow();
});

document.querySelector('#yes')!.addEventListener('click', async () => {
  try {
    await ipc.onboardingChoice(true);
    await ipc.closeWindow();
  } catch (e) {
    const err = document.querySelector<HTMLElement>('#err')!;
    const manual = (await ipc.getSettings()).manual;
    err.hidden = false;
    err.textContent = `Couldn't take over the shortcuts: ${String(e)}. Bind these yourself:`;
    for (const [k, c] of manual) {
      const code = document.createElement('code');
      code.textContent = `${k} → ${c}`;
      err.appendChild(code);
    }
    const yes = document.querySelector<HTMLButtonElement>('#yes')!;
    yes.textContent = 'Close';
    yes.onclick = () => void ipc.closeWindow();
  }
});
```

In `vite.config.ts`, set `const pages = ['overlay', 'countdown', 'thumbnail', 'settings', 'onboarding'];`.

- [ ] **Step 7: Verify**

Run `npm run build && npm test`: no errors.

First run:

```bash
mv ~/.config/rshot/config.toml /tmp/rshot-config.bak 2>/dev/null
npm run tauri dev > /tmp/rshot-dev.log 2>&1 &
```

The onboarding dialog appears, matching mockup 05b.
- Click **Use rshot**. The dialog closes, `gsettings get org.gnome.shell.keybindings show-screenshot-ui` returns `@as []`, `~/.config/autostart/rshot.desktop` exists, and `Print` opens the overlay.

Settings (tray → **Settings…**), matching mockup 05c:
- Each toggle persists. Check `config.toml`, and for launch at login check that the autostart file appears and disappears.
- **Path only** makes the next capture's clipboard TARGETS contain no `image/png`.
- The folder picker changes where the next capture is saved.
- Rebinding **Capture screen** works: click the keycaps, press `Ctrl+Shift+3`, and the keycaps update. `gsettings get …/rshot-screen/ binding` returns `'<Ctrl><Shift>3'`, and pressing `Ctrl+Shift+3` captures.
- Turning **Take over** off restores GNOME's bindings (the `@as []` value becomes `['Print']` again).
- **Open config file** opens `config.toml`.
- The tray items **Open Last Capture** and **Open Screenshots Folder** work.

Finally, restore your config: `mv /tmp/rshot-config.bak ~/.config/rshot/config.toml 2>/dev/null`. Then leave takeover in whatever state you want for daily use.

- [ ] **Step 8: Commit**

```bash
git add -A src src-tauri vite.config.ts && git commit -m "feat: onboarding, settings window and full tray menu

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: CI on Linux, Windows and macOS

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: any file that makes `cargo clippy -D warnings` fail on a platform

**Interfaces:** none. The deliverable is a workflow file plus a warning-free build.

- [ ] **Step 1: Make the local build warning-free**

Run: `cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings`

Expected: no warnings. Fix each one where it lives: remove anything genuinely unused, or gate Linux-only items with `#[cfg(target_os = "linux")]`. Never add a blanket `allow`.

- [ ] **Step 1b: Cross-check Windows and macOS locally**

`cargo check` needs no linking, so rshot can be type-checked for other OSes from this machine. (This was verified on this machine: `llvm-rc` handles Windows resources and `clang` handles the Objective-C helper.) Write `scripts/cross-check.sh`:

```sh
#!/bin/sh
# Type-checks (clippy, no linking) rshot for Windows and macOS from Linux, so cfg/compile errors
# surface before CI. Needs: rustup targets x86_64-pc-windows-msvc + aarch64-apple-darwin, clang, llvm-rc.
set -eu
cd "$(dirname "$0")/../src-tauri"
bin=$(mktemp -d)
trap 'rm -rf "$bin"' EXIT
ln -s "$(command -v llvm-rc || command -v llvm-rc-18)" "$bin/llvm-rc"
export PATH="$bin:$PATH" CC_aarch64_apple_darwin=clang CC_x86_64_apple_darwin=clang
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/cross}"
for t in x86_64-pc-windows-msvc aarch64-apple-darwin; do
  rustup target add "$t" >/dev/null 2>&1 || true
  cargo clippy --target "$t" --all-targets -- -D warnings
done
echo "cross-check OK"
```

Run: `chmod +x scripts/cross-check.sh && scripts/cross-check.sh`
Expected: `cross-check OK`. Fix every error and warning it reports, usually by gating Linux-only items with `#[cfg(target_os = "linux")]` or giving other OSes a stub. Add `scripts/cross-check.sh` to the Step 3 commit.

- [ ] **Step 2: Write `.github/workflows/ci.yml`**

```yaml
name: ci
on:
  push:
  pull_request:

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-22.04, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - name: Linux system libraries
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev \
            librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev \
            libwayland-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev
      - run: npm ci
      - run: npm test
      - run: npm run build
      - name: cargo fmt
        run: cargo fmt --check
        working-directory: src-tauri
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
        working-directory: src-tauri
      - name: cargo test
        run: cargo test
        working-directory: src-tauri
```

- [ ] **Step 3: Check the YAML parses, then commit**

```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"
git add .github src-tauri scripts/cross-check.sh && git commit -m "ci: fmt, clippy, tests and build on Linux, Windows and macOS

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(The workflow first runs when the branch is pushed at the end of Plan 4. Windows and macOS compile errors that show up then are fixed as the first step of Plan 4.)

---

### Task 13: Debian package, uninstall restore, README, full checklist

**Files:**
- Create: `src-tauri/deb/prerm.sh`, `README.md`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:** none. Deliverables: `src-tauri/target/release/bundle/deb/rshot_0.1.0_amd64.deb` installs and works, and `README.md`.

- [ ] **Step 1: Write the pre-remove script**

This script restores shortcuts on uninstall. It runs for every logged-in user, but not on upgrade.

`src-tauri/deb/prerm.sh`:

```sh
#!/bin/sh
# Give the screenshot shortcuts back to GNOME for every logged-in user (best effort).
# Users who aren't logged in can run `gsettings reset org.gnome.shell.keybindings show-screenshot-ui` etc. (see README).
set -e
case "$1" in
  remove|purge)
    for uid in $(loginctl list-users --no-legend 2>/dev/null | awk '{print $1}'); do
      user=$(id -nu "$uid" 2>/dev/null) || continue
      home=$(getent passwd "$user" | cut -d: -f6)
      runuser -u "$user" -- env HOME="$home" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
        /usr/bin/rshot restore-shortcuts || true
    done
    ;;
esac
exit 0
```

Run: `chmod +x src-tauri/deb/prerm.sh && sh -n src-tauri/deb/prerm.sh && echo syntax-ok`

- [ ] **Step 2: Declare the Debian metadata**

In `tauri.conf.json`, inside `"bundle"`, add:

```json
"linux": {
  "deb": {
    "depends": ["libpipewire-0.3-0t64 | libpipewire-0.3-0", "libgbm1"],
    "preRemoveScript": "deb/prerm.sh"
  }
}
```

- [ ] **Step 3: Build and install the package**

```bash
npm run tauri build
ls src-tauri/target/release/bundle/deb/
sudo apt install -y ./src-tauri/target/release/bundle/deb/rshot_0.1.0_amd64.deb
dpkg -s rshot | grep -E 'Status|Depends'
```

Expected:
- `rshot_0.1.0_amd64.deb` is listed.
- `Status: install ok installed`.
- `Depends` includes libwebkit2gtk, libpipewire and libgbm1.

(Ask the user to run the `sudo` line if you can't.)

- [ ] **Step 4: Run the Plan-1 manual checklist on the installed build**

Quit any dev instance first. Launch **rshot** from the app grid, and go through onboarding with **Use rshot**. Then tick every line:

- [ ] `Print` → overlay on all 3 monitors, and the toolbar on the monitor under the pointer. It appears in ≤ 250 ms (it feels instant).
- [ ] Area capture on DP-4 (2560×1600): the PNG size matches the label.
- [ ] Area capture on DP-1 (1920×1080) and HDMI-0: the files are correct and the monitor is correct.
- [ ] Window mode (`Space`) captures a window partly covered by another; the saved image is complete.
- [ ] `Shift+Print` saves the full monitor under the pointer. `Alt+Print` saves the focused window.
- [ ] Clipboard, path + image mode:
  - pasting in gnome-terminal (Ctrl+Shift+V) gives the absolute path;
  - pasting into a Chrome/Slack message box gives the image;
  - pasting in Files (Ctrl+V) gives a copy of the file.
- [ ] Clipboard, path only: pasting in Chrome gives the path text.
- [ ] The thumbnail matches mockup 01-A. Open, reveal, delete, swipe and drag-into-Chrome all work.
- [ ] Timer 3 s: counts down on the live screen and captures a menu opened during the countdown.
- [ ] "Show mouse pointer" on: the pointer appears in the next capture.
- [ ] Settings: every control persists, and a rebind works with the new key.
- [ ] Log out and back in: rshot is in the tray (autostart), and `Print` works on the first press.
- [ ] Remove the package:
  - `sudo apt remove rshot`, then `gsettings get org.gnome.shell.keybindings show-screenshot-ui` → `['Print']` (restored by prerm).
  - Reinstall afterwards for daily use: `sudo apt install ./src-tauri/target/release/bundle/deb/rshot_0.1.0_amd64.deb`, then onboarding again.

If a line fails, fix it in the task that owns it, rebuild, and re-run the whole checklist.

- [ ] **Step 5: Write `README.md`**

````markdown
# rshot

Screenshot tool for Linux (Windows and macOS next) that replaces your OS screenshot tool,
shortcuts included. Every capture puts its **absolute file path** on the clipboard, next to
the file and the image, so a terminal or an AI agent gets a path and a chat app gets the picture.

## Install (Ubuntu / Debian)

```bash
sudo apt install ./rshot_0.1.0_amd64.deb
```

Start **rshot** once from the app grid and choose **Use rshot**.

| Shortcut | Action |
|---|---|
| `Print` | Overlay: select an area, window (`Space`) or screen; `⏎` captures, `Esc` cancels |
| `Shift+Print` | Capture the monitor under the pointer |
| `Alt+Print` | Capture the focused window |

Captures are saved as `~/Pictures/Screenshots/Screenshot_YYYY-MM-DD_HH-MM-SS.png`.
Settings live in the tray menu and in `~/.config/rshot/config.toml`.

## Give the shortcuts back

Turn off **Settings → Take over system screenshot shortcuts**, or run:

```bash
rshot restore-shortcuts
```

Uninstalling the `.deb` does this for every logged-in user. If it was removed while you were
logged out:

```bash
gsettings reset org.gnome.shell.keybindings show-screenshot-ui
gsettings reset org.gnome.shell.keybindings screenshot
gsettings reset org.gnome.shell.keybindings screenshot-window
```

## Other desktops

Automatic takeover works on GNOME. Elsewhere, bind `rshot capture area|screen|window` in your
desktop's keyboard settings.

## Develop

Prerequisites: Rust stable, Node 22, and on Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev libwayland-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev
npm ci
npm run tauri dev        # run
npm test                 # frontend unit tests
(cd src-tauri && cargo test)
npm run tauri build      # .deb in src-tauri/target/release/bundle/deb
```

Design: `docs/superpowers/specs/2026-09-22-rshot-design.md`; mockups in `docs/design/mockups/`.
````

- [ ] **Step 6: Commit**

```bash
git add README.md src-tauri && git commit -m "build: Debian package with shortcut restore on uninstall; README

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Self-review notes (for the executor)

- **Spec coverage (Plan 1 scope):**

  | Spec section | Covered by |
  |---|---|
  | §2.1 shortcuts, Linux column (record from Plan 3) | Tasks 10 + 11 |
  | §2.2 overlay | Tasks 5, 7, 8 |
  | §2.3 after capture | Tasks 4, 9 |
  | §2.4 clipboard | Task 3 |
  | §2.8 tray, first run, settings (no Recording group) | Task 11 |
  | §3.2 process model | Task 1 |
  | §3.6 GNOME takeover | Task 10 |
  | §3.7 Linux `.deb` | Task 13 |
  | §4 error rows for grab, write and clipboard failures | Tasks 4, 9 |
  | §4 takeover error row | Task 11 |
  | §5 performance targets | Task 6 |
  | §6 tests | Tasks 1–4, 7, 10, 11 (units) and Task 13 (manual) |

- **Deferred to later plans:**
  - §2.5 editor: Plan 2.
  - §2.6/§2.7 recording and video: Plan 3.
  - The Windows and macOS rows of §2.1/§3.6/§3.7: Plan 4.
