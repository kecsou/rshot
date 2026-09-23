# rshot Plan 4: Windows + macOS Support, Installers, Release — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** rshot builds, packages and behaves correctly on Windows 10/11 and macOS 12+:
- shortcut takeover (with restore);
- screenshots and recordings with a bundled ffmpeg;
- tray behaviour native to each OS;
- self-sufficient installers (`.deb`, `.rpm`, AppImage, NSIS `.exe`, `.dmg`) built by a GitHub Actions release workflow.

**Architecture:**
- Platform differences sit behind `#[cfg(target_os = …)]` in the modules that already own each concern:
  - `shortcuts/` gains `windows.rs` (registry + low-level keyboard hook) and `macos.rs` (symbolic hotkeys + global shortcuts);
  - `recorder.rs` gains gdigrab/dshow and avfoundation argument builders;
  - `pipeline.rs`, `ui.rs` and `capture.rs` gain small per-OS branches.
- A shared pure `combo.rs` parses the neutral `"Ctrl+Alt+Shift+R"` shortcut strings into modifiers + key for both OSes.
- Nothing here can run on this Linux machine. Correctness is checked by unit tests of the pure parts, by `scripts/cross-check.sh` (clippy for `x86_64-pc-windows-msvc` and `aarch64-apple-darwin`), and later by the user's smoke tests on real machines.

**Tech Stack:**
- Plans 1–3.
- `windows` 0.61 (Win32: hooks, registry, `PlaySoundW`).
- `tauri-plugin-global-shortcut` 2.3 (macOS only).
- ffmpeg sidecars: BtbN win64 GPL 8.1 and Martin Riedl macOS arm64/amd64 release builds.
- `tauri-apps/tauri-action` for releases.

**Spec:** `docs/superpowers/specs/2026-09-22-rshot-design.md` §2.1 (Windows/macOS columns), §2.8 (macOS permission step), §3.6 (Windows/macOS takeover), §3.7 (packaging), §6 (Windows/macOS are compiled and packaged, and the owner smoke-tests them).

## Global Constraints

- Plans 1–3's Global Constraints and Environment notes apply.
- **After every task**, `scripts/cross-check.sh` must print `cross-check OK`, alongside the Linux `cargo clippy --all-targets -- -D warnings`, `cargo test` and `npm test`.
- Default shortcuts:

  | OS | area | screen | window | record | Hard-wired (also open the area overlay) |
  |---|---|---|---|---|---|
  | Linux | `Print` | `Shift+Print` | `Alt+Print` | `Ctrl+Alt+Shift+R` | — |
  | Windows | `Super+Shift+S` | `Super+Print` | `Alt+Print` | `Super+Shift+R` | `Print` |
  | macOS | `Super+Shift+4` | `Super+Shift+3` | `""` (unbound) | `""` (unbound) | `⌘⇧5` |

  On macOS, `⌃⌘⇧3` and `⌃⌘⇧4` map to the same actions as `⌘⇧3` and `⌘⇧4`.
- Windows takeover:
  - Save and set `HKCU\Control Panel\Keyboard\PrintScreenKeyForSnippingEnabled = 0`.
  - A `WH_KEYBOARD_LL` hook swallows the bound combos. After swallowing a `Super` combo it sends a dummy key (`0xE8`) so releasing `Win` doesn't open Start.
  - Restore puts back the saved value (or deletes the value if it didn't exist) and disables the hook.
- macOS takeover:
  - Disable symbolic hotkeys 28, 29, 30, 31 and 184 via `defaults write com.apple.symbolichotkeys`, then run `activateSettings -u`, then register global shortcuts.
  - Restore re-enables the five hotkeys and unregisters the global shortcuts.
- The sidecar is named `rshot-ffmpeg` on every OS (`.exe` on Windows). There's no system ffmpeg dependency anywhere.
- Installers are self-sufficient:
  - The NSIS installer embeds the WebView2 bootstrapper and installs per user. Uninstall runs `rshot.exe restore-shortcuts`.
  - The macOS `.dmg` is built per architecture (arm64, x86_64), unsigned. `Info.plist` has `NSMicrophoneUsageDescription`.
  - AppImage bundles its libraries.
- Deliberate limitations, surfaced in the README:
  - "Show mouse pointer" in screenshots works on Linux only; the toggle is hidden elsewhere. Recordings always show the pointer.
  - macOS builds are unsigned: Gatekeeper warns, and users run `xattr -dr com.apple.quarantine /Applications/rshot.app`.
  - Windows and macOS behaviour is compiled and packaged but hasn't been run by the implementer.

## File Map

```
src-tauri/src/combo.rs                     # neutral combo parser (pure, tested)
src-tauri/src/shortcuts/{mod.rs,windows.rs,macos.rs}
src-tauri/src/store.rs                     # per-OS default Shortcuts
src-tauri/src/recorder.rs                  # gdigrab/dshow + avfoundation args, dshow/avfoundation device parsers
src-tauri/src/overlay.rs                   # Region carries screen index + local rect
src-tauri/src/pipeline.rs                  # shutter sound on Windows/macOS
src-tauri/src/ui.rs                        # macOS simple-fullscreen overlays, tray template icon, tooltip timer
src-tauri/src/capture.rs, settings.rs      # screen-recording permission (macOS), platform info
src-tauri/icons/tray-template.png          # macOS template tray icon
src-tauri/tauri.windows.conf.json, tauri.macos.conf.json, Info.plist, windows/hooks.nsh
scripts/fetch-ffmpeg.sh                    # + Windows, macOS
src/onboarding/*, src/settings/*, src/overlay/options.ts   # platform-aware copy/controls
.github/workflows/{ci.yml,release.yml}
README.md
```

---

### Task 1: Neutral shortcut combos + per-OS defaults

**Files:**
- Create: `src-tauri/src/combo.rs`
- Modify: `src-tauri/src/store.rs` (`Shortcuts::default` per OS), `src-tauri/src/main.rs` (`mod combo;`), `src/settings/main.ts` (show "Not set" for empty shortcuts)

**Interfaces:**
- Produces:
  - `combo::Key { Print, Letter(char), Digit(u8), F(u8) }` and `combo::Combo { ctrl, alt, shift, sup: bool, key: Key }` (Copy, PartialEq, Debug).
  - `combo::parse(&str) -> Option<Combo>`; `""` → `None`.
  - Windows only: `combo::vk(Key) -> u32`.

- [ ] **Step 1: Failing tests** — `src-tauri/src/combo.rs`:

```rust
//! Neutral shortcut strings ("Ctrl+Alt+Shift+R", "Super+Print") ↔ modifiers + key.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Print,
    Letter(char),
    Digit(u8),
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combo {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub sup: bool,
    pub key: Key,
}

pub fn parse(_s: &str) -> Option<Combo> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn c(ctrl: bool, alt: bool, shift: bool, sup: bool, key: Key) -> Combo {
        Combo { ctrl, alt, shift, sup, key }
    }

    #[test]
    fn parses_neutral_combos() {
        assert_eq!(parse("Print"), Some(c(false, false, false, false, Key::Print)));
        assert_eq!(parse("Ctrl+Alt+Shift+R"), Some(c(true, true, true, false, Key::Letter('R'))));
        assert_eq!(parse("Super+Shift+4"), Some(c(false, false, true, true, Key::Digit(4))));
        assert_eq!(parse("Alt+F12"), Some(c(false, true, false, false, Key::F(12))));
    }

    #[test]
    fn rejects_empty_and_unknown() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("Ctrl+"), None);
        assert_eq!(parse("Hyper+X"), None);
        assert_eq!(parse("Ctrl+Space"), None);
    }
}
```

Add `mod combo;` to `main.rs`. Run `cargo test combo` → both FAIL.

- [ ] **Step 2: Implement**

```rust
pub fn parse(s: &str) -> Option<Combo> {
    let parts: Vec<&str> = s.split('+').collect();
    let (key, mods) = parts.split_last()?;
    let mut c = Combo { ctrl: false, alt: false, shift: false, sup: false, key: parse_key(key)? };
    for m in mods {
        match *m {
            "Ctrl" => c.ctrl = true,
            "Alt" => c.alt = true,
            "Shift" => c.shift = true,
            "Super" => c.sup = true,
            _ => return None,
        }
    }
    Some(c)
}

fn parse_key(k: &str) -> Option<Key> {
    let mut chars = k.chars();
    match (chars.next(), chars.next()) {
        _ if k == "Print" => Some(Key::Print),
        (Some(ch), None) if ch.is_ascii_uppercase() => Some(Key::Letter(ch)),
        (Some(ch), None) if ch.is_ascii_digit() => Some(Key::Digit(ch as u8 - b'0')),
        (Some('F'), Some(_)) => k[1..].parse().ok().filter(|n| (1..=24).contains(n)).map(Key::F),
        _ => None,
    }
}

/// Windows virtual-key code.
#[cfg(target_os = "windows")]
pub fn vk(k: Key) -> u32 {
    match k {
        Key::Print => 0x2C,
        Key::Letter(c) => c as u32,
        Key::Digit(d) => u32::from(b'0' + d),
        Key::F(n) => 0x70 + u32::from(n) - 1,
    }
}
```

Run `cargo test combo` → `2 passed`.

- [ ] **Step 3: Per-OS default shortcuts**

In `store.rs`, replace `impl Default for Shortcuts` with:

```rust
impl Default for Shortcuts {
    fn default() -> Self {
        let (area, screen, window, record) = if cfg!(target_os = "windows") {
            ("Super+Shift+S", "Super+Print", "Alt+Print", "Super+Shift+R")
        } else if cfg!(target_os = "macos") {
            ("Super+Shift+4", "Super+Shift+3", "", "")
        } else {
            ("Print", "Shift+Print", "Alt+Print", "Ctrl+Alt+Shift+R")
        };
        Self { area: area.into(), screen: screen.into(), window: window.into(), record: record.into() }
    }
}
```

In `src/settings/main.ts`, render an empty shortcut as `<kbd>Not set</kbd>`: `const v = s.shortcuts[...]; b.innerHTML = v ? v.split('+')…join('') : '<kbd>Not set</kbd>';`.

- [ ] **Step 4: Verify and commit**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && npm test && npm run build && scripts/cross-check.sh`
Expected: all green, and `cross-check OK`. (`combo::parse` is still unused on Linux; add `#[cfg_attr(target_os = "linux", allow(dead_code))]` to `parse`, `parse_key`, `Combo` and `Key` only if clippy complains, with the comment `// used by the Windows/macOS takeover modules`.)

```bash
git add src-tauri src/settings && git commit -m "feat: neutral shortcut combo parser and per-OS default shortcuts

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Windows shortcut takeover (registry + low-level keyboard hook)

**Files:**
- Create: `src-tauri/src/shortcuts/windows.rs`
- Modify: `src-tauri/src/shortcuts/mod.rs`, `src-tauri/src/main.rs` (start the hook on startup when `takeover`), `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `combo::{parse, vk, Combo}`, `cli::Cmd`, `crate::dispatch`, and `Config.{shortcuts, takeover, gnome_backup}`.
  - `gnome_backup` is reused as the generic OS backup map. Add this doc comment to the field: `/// Original OS shortcut settings (GNOME keybindings / Windows registry), kept while rshot owns them.`
- Produces (Windows only):
  - `shortcuts::windows::{take_over(&mut Config) -> Result<(), String>, restore(&mut Config) -> Result<(), String>, install_hook(AppHandle), set_bindings(&Shortcuts)}`.
  - `shortcuts::take_over`/`restore` route to it on Windows.
  - `shortcuts::start(&AppHandle, &Config)` is a no-op on Linux; on Windows/macOS it activates bindings when `takeover` is set.

- [ ] **Step 1: Dependency** (Windows only)

```bash
cargo add windows@0.61 --target 'cfg(windows)' --features Win32_Foundation,Win32_UI_WindowsAndMessaging,Win32_UI_Input_KeyboardAndMouse,Win32_System_Registry,Win32_Media_Audio
```

- [ ] **Step 2: Write `shortcuts/windows.rs`**

```rust
//! Windows: stop PrtScn opening Snipping Tool (registry), then own the combos with a low-level
//! keyboard hook (RegisterHotKey can't claim Win+Shift+S, which the shell reserves).

use crate::{cli::Cmd, combo, store::{Config, Shortcuts}};
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Mutex, OnceLock};
use tauri::AppHandle;
use windows::{
    core::w,
    Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        System::Registry::{RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD},
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT},
            WindowsAndMessaging::{CallNextHookEx, GetMessageW, SetWindowsHookExW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN},
        },
    },
};

const BACKUP_KEY: &str = "win:PrintScreenKeyForSnippingEnabled";
static ENABLED: AtomicBool = AtomicBool::new(false);
static BINDINGS: Mutex<Vec<(combo::Combo, Cmd)>> = Mutex::new(Vec::new());
static SENDER: OnceLock<Mutex<mpsc::Sender<Cmd>>> = OnceLock::new();

fn read_snipping() -> Option<u32> {
    let mut v: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, w!("Control Panel\\Keyboard"), w!("PrintScreenKeyForSnippingEnabled"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut size))
    }
    .ok()
    .ok()
    .map(|_| v)
}

fn write_snipping(v: u32) -> Result<(), String> {
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, w!("Control Panel\\Keyboard"), w!("PrintScreenKeyForSnippingEnabled"), REG_DWORD.0, Some(&v as *const u32 as *const _), 4)
    }
    .ok()
    .map_err(|e| e.to_string())
}

/// The combos rshot owns: the configured ones plus PrtScn → area (always, while taken over).
pub fn set_bindings(s: &Shortcuts) {
    let mut b = vec![];
    for (text, cmd) in [(&s.area, Cmd::CaptureArea), (&s.screen, Cmd::CaptureScreen), (&s.window, Cmd::CaptureWindow), (&s.record, Cmd::Record)] {
        if let Some(c) = combo::parse(text) {
            b.push((c, cmd));
        }
    }
    b.push((combo::Combo { ctrl: false, alt: false, shift: false, sup: false, key: combo::Key::Print }, Cmd::CaptureArea));
    *BINDINGS.lock().unwrap() = b;
}

pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    let backup = cfg.gnome_backup.get_or_insert_with(Default::default);
    backup.entry(BACKUP_KEY.into()).or_insert_with(|| read_snipping().map(|v| v.to_string()).unwrap_or_default());
    write_snipping(0)?;
    set_bindings(&cfg.shortcuts);
    ENABLED.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn restore(cfg: &mut Config) -> Result<(), String> {
    ENABLED.store(false, Ordering::SeqCst);
    if let Some(v) = cfg.gnome_backup.as_mut().and_then(|b| b.remove(BACKUP_KEY)) {
        match v.parse::<u32>() {
            Ok(n) => write_snipping(n)?,
            Err(_) => unsafe {
                let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, w!("Control Panel\\Keyboard"), w!("PrintScreenKeyForSnippingEnabled"));
            },
        }
    }
    if cfg.gnome_backup.as_ref().is_some_and(|b| b.is_empty()) {
        cfg.gnome_backup = None;
    }
    Ok(())
}

fn down(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0
}

/// Keeps Start from opening when Win is released after a swallowed Win-combo.
fn mask_win() {
    let key = |flags| INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0xE8), dwFlags: flags, ..Default::default() } } };
    unsafe {
        SendInput(&[key(Default::default()), key(KEYEVENTF_KEYUP)], std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let msg = wparam.0 as u32;
    if code == HC_ACTION as i32 && ENABLED.load(Ordering::Relaxed) && (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let (ctrl, alt, shift, sup) = (down(VK_CONTROL), down(VK_MENU), down(VK_SHIFT), down(VK_LWIN) || down(VK_RWIN));
        let hit = BINDINGS.lock().ok().and_then(|b| {
            b.iter()
                .find(|(c, _)| combo::vk(c.key) == kb.vkCode && c.ctrl == ctrl && c.alt == alt && c.shift == shift && c.sup == sup)
                .map(|(c, cmd)| (c.sup, *cmd))
        });
        if let Some((with_win, cmd)) = hit {
            if with_win {
                mask_win();
            }
            if let Some(tx) = SENDER.get() {
                let _ = tx.lock().unwrap().send(cmd);
            }
            return LRESULT(1); // swallow
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Hook thread (needs a message loop) + a worker that runs the commands off the hook.
pub fn install_hook(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<Cmd>();
    if SENDER.set(Mutex::new(tx)).is_err() {
        return; // already installed
    }
    std::thread::spawn(move || {
        for cmd in rx {
            crate::dispatch(&app, cmd);
        }
    });
    std::thread::spawn(|| unsafe {
        if SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0).is_err() {
            eprintln!("rshot: keyboard hook failed");
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
}
```

The `windows` crate's exact signatures (which parameters are `Option`, whether a call returns `Result` or `WIN32_ERROR`) differ between versions. Match them to 0.61 using the compiler errors from `scripts/cross-check.sh`, keeping the logic unchanged.

- [ ] **Step 3: Route `shortcuts` by OS, and start at launch**

In `shortcuts/mod.rs`:
- Add `#[cfg(target_os = "windows")] mod windows;`.
- Make the non-Linux `take_over`/`restore` stubs `#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]`.
- Add:

```rust
#[cfg(target_os = "windows")]
pub fn take_over(cfg: &mut Config) -> Result<(), String> {
    windows::take_over(cfg)
}

#[cfg(target_os = "windows")]
pub fn restore(cfg: &mut Config) -> Result<(), String> {
    windows::restore(cfg)
}

/// Called once at startup: Windows/macOS bindings live in-process, so re-arm them.
pub fn start(app: &tauri::AppHandle, cfg: &mut Config) {
    #[cfg(target_os = "windows")]
    {
        windows::install_hook(app.clone());
        if cfg.takeover {
            let _ = windows::take_over(cfg);
        }
    }
    let _ = (app, cfg);
}
```

(Task 3 adds the macOS arm of `start`.)

In `main.rs` `setup`, after `ensure_overlays`, add:

```rust
{
    let state = app.state::<AppState>();
    let mut c = state.config.lock().unwrap();
    shortcuts::start(app.handle(), &mut c);
}
```

In `settings::set_settings`, when takeover stays on and shortcuts changed, the existing `take_over_or_roll_back` call refreshes the bindings (`take_over` calls `set_bindings`). Nothing else changes.

- [ ] **Step 4: Verify and commit**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && scripts/cross-check.sh`
Expected: green, and `cross-check OK`. (There's no runtime test on this machine.)

```bash
git add src-tauri && git commit -m "feat(windows): take over PrtScn/Win+Shift+S via registry and low-level keyboard hook

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: macOS — shortcut takeover, overlays, tray, permission, sound

**Files:**
- Create: `src-tauri/src/shortcuts/macos.rs`, `src-tauri/icons/tray-template.png`, `src-tauri/Info.plist`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/shortcuts/mod.rs`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`, `src-tauri/src/capture.rs`, `src-tauri/src/pipeline.rs`, `src-tauri/src/settings.rs`, `src/onboarding/{index.html,main.ts}`, `src/shared/ipc.ts`

**Interfaces:**
- Produces:
  - (macOS) `shortcuts::macos::{take_over, restore, register(&AppHandle, &Shortcuts)}`.
  - `capture::{screen_permission() -> bool, request_screen_permission()}`, which return `true` / do nothing off macOS.
  - `Settings.platform: String` (`"linux"|"windows"|"macos"`) and `Settings.screen_permission: bool`.
  - Command `request_screen_permission`.

- [ ] **Step 1: Dependencies**

```bash
cargo add tauri-plugin-global-shortcut@2.3 --target 'cfg(target_os = "macos")'
cargo add tauri@2.11 --features tray-icon,macos-private-api,protocol-asset,image-png
```

- [ ] **Step 2: `shortcuts/macos.rs`**

```rust
//! macOS: switch off the system screenshot hotkeys, then own ⌘⇧3/4/5 (and the ⌃ variants) as global shortcuts.

use crate::{cli::Cmd, combo, store::{Config, Shortcuts}};
use std::process::Command;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// (symbolic hotkey id, ASCII, virtual key code, modifier mask) for ⌘⇧3, ⌃⌘⇧3, ⌘⇧4, ⌃⌘⇧4, ⌘⇧5.
const SYSTEM: [(u32, u32, u32, u32); 5] =
    [(28, 51, 20, 1179648), (29, 51, 20, 1441792), (30, 52, 21, 1179648), (31, 52, 21, 1441792), (184, 53, 23, 1179648)];

fn set_system(enabled: bool) -> Result<(), String> {
    for (id, ascii, vk, mods) in SYSTEM {
        let value = format!("{{enabled = {}; value = {{ parameters = ({ascii}, {vk}, {mods}); type = standard; }}; }}", u8::from(enabled));
        let ok = Command::new("defaults")
            .args(["write", "com.apple.symbolichotkeys", "AppleSymbolicHotKeys", "-dict-add", &id.to_string(), &value])
            .status()
            .map_err(|e| e.to_string())?
            .success();
        if !ok {
            return Err(format!("defaults write failed for hotkey {id}"));
        }
    }
    let _ = Command::new("/System/Library/PrivateFrameworks/SystemAdministration.framework/Resources/activateSettings").arg("-u").status();
    Ok(())
}

fn code(k: combo::Key) -> Code {
    match k {
        combo::Key::Print => Code::PrintScreen,
        combo::Key::Digit(d) => [Code::Digit0, Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5, Code::Digit6, Code::Digit7, Code::Digit8, Code::Digit9][d as usize],
        combo::Key::F(n) => [Code::F1, Code::F2, Code::F3, Code::F4, Code::F5, Code::F6, Code::F7, Code::F8, Code::F9, Code::F10, Code::F11, Code::F12][(n as usize - 1).min(11)],
        combo::Key::Letter(c) => format!("Key{c}").parse().unwrap_or(Code::KeyA),
    }
}

fn shortcut(c: combo::Combo) -> Shortcut {
    let mut m = Modifiers::empty();
    m.set(Modifiers::CONTROL, c.ctrl);
    m.set(Modifiers::ALT, c.alt);
    m.set(Modifiers::SHIFT, c.shift);
    m.set(Modifiers::SUPER, c.sup);
    Shortcut::new(Some(m), code(c.key))
}

/// Configured bindings + ⌘⇧5 → area + ⌃ variants of ⌘⇧3/⌘⇧4.
fn bindings(s: &Shortcuts) -> Vec<(Shortcut, Cmd)> {
    let mut v = vec![];
    for (text, cmd) in [(&s.area, Cmd::CaptureArea), (&s.screen, Cmd::CaptureScreen), (&s.window, Cmd::CaptureWindow), (&s.record, Cmd::Record)] {
        if let Some(c) = combo::parse(text) {
            v.push((shortcut(c), cmd));
            if c.sup && c.shift && !c.ctrl && !c.alt {
                v.push((shortcut(combo::Combo { ctrl: true, ..c }), cmd));
            }
        }
    }
    v.push((shortcut(combo::parse("Super+Shift+5").expect("valid")), Cmd::CaptureArea));
    v
}

pub fn register(app: &AppHandle, s: &Shortcuts) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for (sc, cmd) in bindings(s) {
        let _ = gs.on_shortcut(sc, move |app, _sc, event| {
            if event.state == ShortcutState::Pressed {
                crate::dispatch(app, cmd);
            }
        });
    }
}

pub fn take_over(app: &AppHandle, cfg: &mut Config) -> Result<(), String> {
    set_system(false)?;
    register(app, &cfg.shortcuts);
    Ok(())
}

pub fn restore(app: Option<&AppHandle>, _cfg: &mut Config) -> Result<(), String> {
    if let Some(app) = app {
        let _ = app.global_shortcut().unregister_all();
    }
    set_system(true)
}
```

macOS takeover needs the `AppHandle` to register shortcuts, but the shared `shortcuts::take_over(&mut Config)` signature has no handle. So:
- Keep a `static APP: OnceLock<AppHandle>` in `macos.rs`, set by `shortcuts::start`.
- Add wrappers `pub fn take_over_cfg(cfg)` and `pub fn restore_cfg(cfg)` that read it. `restore_cfg` passes `None` when `APP` is unset (the CLI `restore-shortcuts` path).
- Route `shortcuts::take_over`/`restore` to them on macOS, and in `shortcuts::start` add:

```rust
    #[cfg(target_os = "macos")]
    {
        macos::init(app.clone());
        if cfg.takeover {
            let _ = macos::take_over_cfg(cfg);
        }
    }
```

where `pub fn init(app: AppHandle) { let _ = APP.set(app); }`.

In `main.rs`, add `#[cfg(target_os = "macos")] let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());`. Restructure the builder into a `let builder = …;` binding before `.build(...)` if needed.

- [ ] **Step 3: Overlays, tray and dock**

In `ui::place_overlays`, on macOS use simple fullscreen (a borderless window over the menu bar with no new Space or animation) instead of `set_fullscreen`:

```rust
        #[cfg(target_os = "macos")]
        {
            w.set_position(pos).map_err(err)?;
            w.set_size(PhysicalSize::new(f.image.width(), f.image.height())).map_err(err)?;
            w.set_simple_fullscreen(true).map_err(err)?;
            continue;
        }
```

Place it at the top of the loop body, after computing `pos`. The existing branch runs on the other OSes.

Tray:
- Generate `src-tauri/icons/tray-template.png`: a 44×44 black-on-transparent version of the logo's brackets and ring. Use the Plan 1 icon script with `S=44`, black strokes, no gradient, width 4, margins scaled.
- In `create_tray`, on macOS use `tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?` with `.icon_as_template(true)`.
- `set_tray_timer` also sets the tooltip on every OS (`tray.set_tooltip(Some(format!("rshot — recording {m}:{ss}")))`, resetting it to `"rshot"` on `None`), because Windows has no tray title.

In `setup`, add `#[cfg(target_os = "macos")] app.set_activation_policy(tauri::ActivationPolicy::Accessory);` (no Dock icon; tray only).

- [ ] **Step 4: Screen-recording permission + mic usage string**

`src-tauri/Info.plist` (Tauri merges it into the bundle's Info.plist):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSMicrophoneUsageDescription</key>
  <string>rshot records your microphone only when you turn it on for a screen recording.</string>
</dict>
</plist>
```

In `capture.rs`:

```rust
#[cfg(target_os = "macos")]
mod permission {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    pub fn granted() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    pub fn request() {
        unsafe {
            CGRequestScreenCaptureAccess();
        }
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .status();
    }
}

pub fn screen_permission() -> bool {
    #[cfg(target_os = "macos")]
    return permission::granted();
    #[cfg(not(target_os = "macos"))]
    true
}

pub fn request_screen_permission() {
    #[cfg(target_os = "macos")]
    permission::request();
}
```

In `settings.rs`:
- `Settings` gains `#[serde(default)] pub platform: String` and `#[serde(default)] pub screen_permission: bool`.
- `snapshot` fills them in (`std::env::consts::OS.to_string()`, `capture::screen_permission()`).
- Add the command `#[tauri::command] pub fn request_screen_permission() { crate::capture::request_screen_permission(); }` and register it. Also add it to `build.rs`'s `AppManifest` and to `capabilities/dialogs.json` (webview hardening from Plan 3 Task 0).

`ipc.ts`:
- `Settings` gains `platform: string; screen_permission: boolean`.
- Add `export const requestScreenPermission = () => invoke<void>('request_screen_permission');`.

Onboarding, as spec §2.8 and the note under mockup 5b describe:
- The paragraph text becomes platform-neutral: "rshot will take over these shortcuts from your system's screenshot tool and start when you log in. You can give the shortcuts back at any time in Settings."
- Hide rows whose shortcut is empty.
- After a successful `onboardingChoice(true)` on `platform === 'macos' && !screen_permission`, don't close. Swap the dialog to a second step:
  - heading "Allow screen recording";
  - text "macOS asks once. Turn on rshot in System Settings → Privacy & Security → Screen Recording, then come back.";
  - buttons **Open System Settings** (`requestScreenPermission()`) and **Done** (`closeWindow()`).

- [ ] **Step 5: Sounds and the pointer toggle**

`pipeline::play_shutter` (append the two OS versions; keep Linux):

```rust
#[cfg(target_os = "windows")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    use windows::{core::HSTRING, Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME}};
    if let Ok(wav) = app.path().resolve("sounds/shutter.wav", BaseDirectory::Resource) {
        unsafe {
            let _ = PlaySoundW(&HSTRING::from(wav.as_os_str()), None, SND_FILENAME | SND_ASYNC);
        }
    }
}

#[cfg(target_os = "macos")]
fn play_shutter(app: &AppHandle) {
    use tauri::path::BaseDirectory;
    if let Ok(wav) = app.path().resolve("sounds/shutter.wav", BaseDirectory::Resource) {
        std::thread::spawn(move || {
            let _ = std::process::Command::new("afplay").arg(wav).status();
        });
    }
}
```

Remove the old `#[cfg(not(target_os = "linux"))]` no-op.

`OverlayOptions` gains `pointer_supported: bool` (`cfg!(target_os = "linux")`). In `src/overlay/options.ts`, hide the "Show mouse pointer" `.tog` when it's `false`.

- [ ] **Step 6: Verify and commit**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && npm test && npm run build && scripts/cross-check.sh`
Expected: all green, and `cross-check OK`. Also run `npm run tauri dev` on Linux and confirm nothing regressed: Print → overlay, onboarding text.

```bash
git add src-tauri src && git commit -m "feat(macos): symbolic-hotkey takeover, simple-fullscreen overlays, template tray, permission step, sounds

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Recording on Windows and macOS

**Files:**
- Modify: `src-tauri/src/recorder.rs`, `src-tauri/src/overlay.rs` (Region carries the screen index and local rect)
- Test: unit tests in `recorder.rs` for every per-OS builder and parser. The pure functions are compiled on **all** OSes so they're tested on Linux, and dispatch picks one per OS.

**Interfaces:**
- Produces:
  - `Region` gains `screen: usize, lx: u32, ly: u32` (the monitor index in `Monitor::all()` order, and the top-left in that monitor's image pixels).
  - Pure builders:

    ```rust
    gdigrab_args(Region, fps, Option<&str>, &Path) -> Vec<String>
    avfoundation_args(screen_dev: usize, audio_dev: Option<&str>, crop: Option<(u32, u32, u32, u32)>, fps, &Path) -> Vec<String>
    ```

  - Pure parsers:

    ```rust
    parse_dshow_audio(&str) -> Vec<Mic>
    parse_avfoundation(&str) -> (Vec<(usize, usize)> /* screen n → device index */, Vec<Mic>)
    ```

  - `record_args` dispatches by OS. The macOS path looks up the device index with `ffmpeg -f avfoundation -list_devices true -i ""`.

- [ ] **Step 1: Failing tests** (add to `recorder.rs` tests)

```rust
    #[test]
    fn gdigrab_args_offset_and_mic() {
        let r = Region { x: -1920, y: 0, w: 1281, h: 720, screen: 0, lx: 0, ly: 0 };
        let a = gdigrab_args(r, 30, Some("Microphone (Realtek)"), Path::new("C:/v/.R.mkv"));
        assert!(a.windows(4).any(|w| w == s(&["-offset_x", "-1920", "-offset_y", "0"])));
        assert!(a.windows(2).any(|w| w == s(&["-video_size", "1280x720"])));
        assert!(a.windows(4).any(|w| w == s(&["-f", "dshow", "-i", "audio=Microphone (Realtek)"])));
        assert!(a.windows(2).any(|w| w == s(&["-i", "desktop"])));
    }

    #[test]
    fn parses_dshow_audio_devices() {
        let out = "[dshow @ 000001] \"Integrated Camera\" (video)\n[dshow @ 000001]   Alternative name \"@device_pnp_x\"\n[dshow @ 000001] \"Microphone (Realtek(R) Audio)\" (audio)\n[dshow @ 000001]   Alternative name \"@device_cm_y\"\n";
        assert_eq!(parse_dshow_audio(out), vec![Mic { id: "Microphone (Realtek(R) Audio)".into(), label: "Microphone (Realtek(R) Audio)".into() }]);
    }

    #[test]
    fn parses_avfoundation_devices() {
        let out = "[AVFoundation indev @ 0x1] AVFoundation video devices:\n[AVFoundation indev @ 0x1] [0] FaceTime HD Camera\n[AVFoundation indev @ 0x1] [1] Capture screen 0\n[AVFoundation indev @ 0x1] [2] Capture screen 1\n[AVFoundation indev @ 0x1] AVFoundation audio devices:\n[AVFoundation indev @ 0x1] [0] MacBook Pro Microphone\n";
        let (screens, mics) = parse_avfoundation(out);
        assert_eq!(screens, vec![(0, 1), (1, 2)]);
        assert_eq!(mics, vec![Mic { id: "0".into(), label: "MacBook Pro Microphone".into() }]);
    }

    #[test]
    fn avfoundation_args_crop_in_pixels() {
        let a = avfoundation_args(1, Some("0"), Some((10, 20, 641, 480)), 60, Path::new("/v/.R.mkv"));
        assert!(a.windows(2).any(|w| w == s(&["-i", "1:0"])));
        assert!(a.windows(2).any(|w| w == s(&["-vf", "crop=640:480:10:20"])));
        let b = avfoundation_args(1, None, None, 30, Path::new("/o.mkv"));
        assert!(b.windows(2).any(|w| w == s(&["-i", "1:none"])));
        assert!(!b.iter().any(|x| x == "-vf"));
    }
```

Update the existing Linux `record_args` test's `Region` literals to include `screen: 0, lx: 0, ly: 0`. Run `cargo test recorder`: the new tests fail to compile or fail.

- [ ] **Step 2: Implement**

Add the fields to `Region`. In `overlay::overlay_record`:
- Set `screen: index`.
- `lx`/`ly` are the clamped rect's `x`/`y` for area recordings, and `0`/`0` for the full screen.

Add to `recorder.rs`:

```rust
pub fn gdigrab_args(r: Region, fps: u8, mic: Option<&str>, out: &Path) -> Vec<String> {
    let mut a = strs(&["-hide_banner", "-loglevel", "error", "-f", "gdigrab", "-framerate"]);
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
    a.extend(strs(&["-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p"]));
    if mic.is_some() {
        a.extend(strs(&["-c:a", "aac", "-b:a", "160k"]));
    }
    a.push("-y".into());
    a.push(out.display().to_string());
    a
}

pub fn avfoundation_args(screen_dev: usize, audio_dev: Option<&str>, crop: Option<(u32, u32, u32, u32)>, fps: u8, out: &Path) -> Vec<String> {
    let mut a = strs(&["-hide_banner", "-loglevel", "error", "-f", "avfoundation", "-capture_cursor", "1", "-framerate"]);
    a.push(fps.to_string());
    a.push("-i".into());
    a.push(format!("{screen_dev}:{}", audio_dev.unwrap_or("none")));
    if let Some((x, y, w, h)) = crop {
        a.push("-vf".into());
        a.push(format!("crop={}:{}:{x}:{y}", even(w), even(h)));
    }
    a.extend(strs(&["-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p"]));
    if audio_dev.is_some() {
        a.extend(strs(&["-c:a", "aac", "-b:a", "160k"]));
    }
    a.push("-y".into());
    a.push(out.display().to_string());
    a
}

pub fn parse_dshow_audio(out: &str) -> Vec<Mic> {
    out.lines()
        .filter(|l| l.trim_end().ends_with("(audio)"))
        .filter_map(|l| {
            let name = l.split_once('"')?.1.split_once('"')?.0;
            Some(Mic { id: name.into(), label: name.into() })
        })
        .collect()
}

pub fn parse_avfoundation(out: &str) -> (Vec<(usize, usize)>, Vec<Mic>) {
    let (mut screens, mut mics, mut audio) = (vec![], vec![], false);
    for line in out.lines() {
        if line.contains("audio devices:") {
            audio = true;
            continue;
        }
        let Some(rest) = line.split_once("] [").map(|(_, r)| r) else { continue };
        let Some((idx, name)) = rest.split_once("] ") else { continue };
        let Ok(idx) = idx.parse::<usize>() else { continue };
        if audio {
            mics.push(Mic { id: idx.to_string(), label: name.trim().into() });
        } else if let Some(n) = name.trim().strip_prefix("Capture screen ") {
            if let Ok(n) = n.parse() {
                screens.push((n, idx));
            }
        }
    }
    (screens, mics)
}
```

Change `record_args` so it's defined on all OSes. It takes the ffmpeg path for the macOS device lookup:

```rust
pub fn record_args(ffmpeg: &Path, r: Region, display: &str, fps: u8, mic: Option<&str>, out: &Path) -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let _ = ffmpeg;
        return Ok(x11grab_args(r, display, fps, mic, out));
    }
    #[cfg(target_os = "windows")]
    {
        let _ = (ffmpeg, display);
        return Ok(gdigrab_args(r, fps, mic, out));
    }
    #[cfg(target_os = "macos")]
    {
        let _ = display;
        let list = std::process::Command::new(ffmpeg)
            .args(["-hide_banner", "-f", "avfoundation", "-list_devices", "true", "-i", ""])
            .output()
            .map_err(|e| e.to_string())?;
        let (screens, _) = parse_avfoundation(&String::from_utf8_lossy(&list.stderr));
        let dev = screens.iter().find(|(n, _)| *n == r.screen).map(|(_, d)| *d).ok_or("screen not found for recording")?;
        let full = r.lx == 0 && r.ly == 0 && r.x == 0 && r.y == 0;
        let crop = (!full).then_some((r.lx, r.ly, r.w, r.h));
        return Ok(avfoundation_args(dev, mic, crop, fps, out));
    }
}
```

The old Linux `record_args` body becomes `fn x11grab_args(...) -> Vec<String>` (no `Result`). Update the Linux test to call `x11grab_args`. Update `start`'s call site and the ignored integration test.

Make `list_mics` OS-aware:
- Linux: `-sources pulse` → `parse_pulse_sources(stdout)`.
- Windows: `-list_devices true -f dshow -i dummy` → `parse_dshow_audio(stderr)`.
- macOS: `-f avfoundation -list_devices true -i ""` → `parse_avfoundation(stderr).1`.

Gate each parser's *use* by cfg, but keep the parsers themselves compiled everywhere so the tests run on Linux. Add `#[cfg_attr(target_os = "linux", allow(dead_code))]` with a short comment where clippy flags the non-Linux builders and parsers as unused on Linux.

On macOS `full` should really come from the stored `full` flag, but `Region` doesn't carry it. Pass it into `record_args` as an extra parameter, `full: bool`, from `start`, and use `(!full).then_some(...)`. Update the signature, the call site and the tests accordingly.

- [ ] **Step 3: Verify and commit**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && scripts/cross-check.sh`, then a Linux recording smoke test (`rshot record`, 3 s, Stop → MP4 plays).

```bash
git add src-tauri && git commit -m "feat(record): gdigrab/dshow and avfoundation recording with tested argument builders and device parsers

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Sidecars for all OSes, installer configuration, release workflow

**Files:**
- Modify: `scripts/fetch-ffmpeg.sh`, `src-tauri/tauri.conf.json`, `.github/workflows/ci.yml`, `README.md`
- Create: `src-tauri/tauri.windows.conf.json`, `src-tauri/tauri.macos.conf.json`, `src-tauri/windows/hooks.nsh`, `.github/workflows/release.yml`

**Interfaces:** none (build configuration).

- [ ] **Step 1: Fetch script for every target**

Extend the `case` in `scripts/fetch-ffmpeg.sh`. Before the `case`, set `ext=""; [ "${triple#*windows}" != "$triple" ] && ext=".exe"`, and use `out="binaries/rshot-ffmpeg-$triple$ext"` everywhere.

```sh
  x86_64-pc-windows-msvc)
    curl -fsSL -o "$tmp/ff.zip" https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.1-latest-win64-gpl-8.1.zip
    unzip -q "$tmp/ff.zip" -d "$tmp"
    cp "$tmp"/ffmpeg-*/bin/ffmpeg.exe "$out"
    grab=gdigrab
    ;;
  aarch64-apple-darwin|x86_64-apple-darwin)
    arch=arm64; [ "$triple" = x86_64-apple-darwin ] && arch=amd64
    curl -fsSL -o "$tmp/ff.zip" "https://ffmpeg.martin-riedl.de/redirect/latest/macos/$arch/release/ffmpeg.zip"
    unzip -q "$tmp/ff.zip" -d "$tmp"
    cp "$tmp/ffmpeg" "$out"
    grab=avfoundation
    ;;
```

Set `grab=x11grab` in the Linux branch. Replace the x11grab capability check with a `$grab` check. Skip running the binary when it can't run on the build host: only run the checks if `uname -s` matches the target family (Linux/`MINGW*|MSYS*`/Darwin). Also accept a `TARGET` environment override for the triple (`triple=${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}`), so macOS CI can fetch the x86_64 build on an arm64 runner.

- [ ] **Step 2: Platform config files**

`src-tauri/tauri.windows.conf.json`:

```json
{
  "bundle": {
    "targets": ["nsis"],
    "externalBin": ["binaries/rshot-ffmpeg"],
    "windows": {
      "webviewInstallMode": { "type": "embedBootstrapper" },
      "nsis": { "installMode": "currentUser", "installerHooks": "windows/hooks.nsh" }
    }
  }
}
```

`src-tauri/windows/hooks.nsh`:

```nsis
!macro NSIS_HOOK_PREUNINSTALL
  ; Give PrtScn / Win+Shift+S back to Windows before removing rshot.
  ExecWait '"$INSTDIR\rshot.exe" restore-shortcuts'
!macroend
```

`src-tauri/tauri.macos.conf.json`:

```json
{
  "bundle": {
    "targets": ["dmg"],
    "externalBin": ["binaries/rshot-ffmpeg"],
    "macOS": { "minimumSystemVersion": "12.0" }
  }
}
```

In `tauri.conf.json`, keep `"targets": ["deb"]` as the Linux default, and in `src-tauri/tauri.linux.conf.json` add `"targets": ["deb", "rpm", "appimage"]`. For the rpm, add `"linux": { "rpm": { "depends": ["pipewire-libs", "mesa-libgbm"] } }` next to the existing deb block in `tauri.conf.json`.

- [ ] **Step 3: CI and release workflows**

In `ci.yml`, make the sidecar step run on every OS with `shell: bash` (`run: scripts/fetch-ffmpeg.sh`), and remove its `if:`.

`.github/workflows/release.yml`:

```yaml
name: release
on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-14
            target: aarch64-apple-darwin
          - os: macos-14
            target: x86_64-apple-darwin
    runs-on: ${{ matrix.os }}
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: npm }
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: '${{ matrix.target }}' }
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: src-tauri }
      - name: Linux system libraries
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev \
            librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev \
            libwayland-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev rpm
      - name: Bundled ffmpeg sidecar
        shell: bash
        env: { TARGET: '${{ matrix.target }}' }
        run: scripts/fetch-ffmpeg.sh
      - run: npm ci
      - name: Build installers
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          args: --target ${{ matrix.target }}
          tagName: ${{ github.ref_type == 'tag' && github.ref_name || '' }}
          releaseName: 'rshot ${{ github.ref_name }}'
          releaseDraft: true
      - name: Upload installers (manual runs)
        if: github.event_name == 'workflow_dispatch'
        uses: actions/upload-artifact@v4
        with:
          name: rshot-${{ matrix.target }}
          path: |
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.deb
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.rpm
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.AppImage
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.exe
            src-tauri/target/${{ matrix.target }}/release/bundle/**/*.dmg
```

Check the YAML with `python3 -c "import yaml; [yaml.safe_load(open(f)) for f in ('.github/workflows/ci.yml', '.github/workflows/release.yml')]; print('ok')"`.

- [ ] **Step 4: README — install per OS and limitations**

Replace the "Install" section with:

- **Ubuntu/Debian:** `sudo apt install ./rshot_<v>_amd64.deb`.
- **Fedora:** `sudo dnf install ./rshot-<v>-1.x86_64.rpm`.
- **Other Linux:** `chmod +x rshot_<v>_amd64.AppImage && ./rshot_<v>_amd64.AppImage`.
- **Windows:** run `rshot_<v>_x64-setup.exe` (per-user, no admin). SmartScreen may warn because the build is unsigned: More info → Run anyway.
- **macOS:** open the `.dmg` for your chip, drag rshot to Applications, then run `xattr -dr com.apple.quarantine /Applications/rshot.app` (the app is unsigned). On first launch, allow Screen Recording when asked.

Then a per-OS shortcut table (from Global Constraints), and a "Known limitations" list:
- The pointer can be included in screenshots on Linux only.
- macOS and Windows builds are unsigned.
- GNOME/KDE on Wayland aren't supported yet.
- On macOS, uninstalling by dragging to the Trash doesn't restore ⌘⇧3/4/5, so run `/Applications/rshot.app/Contents/MacOS/rshot restore-shortcuts` first.
- Release builds come from `.github/workflows/release.yml`: push a `v*` tag, or run it manually from the Actions tab.

- [ ] **Step 5: Build the Linux installers locally and commit**

```bash
npm run tauri build
ls src-tauri/target/release/bundle/{deb,rpm,appimage}/
```

Expected: `.deb`, `.rpm` and `.AppImage` exist. The AppImage step downloads linuxdeploy; if it complains about FUSE, rerun with `APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri build`. Record any bundle that can't be built here in the report, since CI builds it too.

Then:
- run `dpkg-deb -c …deb | grep rshot-ffmpeg`;
- run `rpm -qlp …rpm 2>/dev/null | grep rshot-ffmpeg` (if `rpm` is available);
- run `scripts/cross-check.sh`.

```bash
git add scripts src-tauri .github README.md && git commit -m "build: bundled ffmpeg for every OS, NSIS/dmg/rpm/AppImage installers, release workflow

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Self-review notes

- **Spec coverage:**

  | Spec item | Covered by |
  |---|---|
  | §2.1 Windows/macOS columns | Tasks 1–3 |
  | §2.8 macOS permission step | Task 3 |
  | §3.6 Windows takeover | Task 2 |
  | §3.6 macOS takeover | Task 3 |
  | §3.7 installers | Task 5 |
  | §3.7 uninstall hook (NSIS) | Task 5 |
  | §3.7 README restore note (macOS) | Task 5 |
  | §3.7 CI | Task 5 |
  | §6 Windows/macOS compiled + packaged | cross-check after every task; release workflow |

- **Not verifiable here:** runtime behaviour on Windows and macOS (hooks, simple fullscreen, avfoundation device mapping, coordinate scaling on Retina displays). This is listed for the owner's smoke test in the final summary.
