//! Windows: stop PrtScn opening Snipping Tool (registry), then own the combos with a low-level
//! keyboard hook (RegisterHotKey can't claim Win+Shift+S, which the shell reserves).
//! Every decision is a plain function, unit-tested on every OS; only `sys` calls Windows.
#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use crate::{cli::Cmd, combo, store::Shortcuts};

#[cfg(target_os = "windows")]
pub use sys::{install_hook, pause, release, restore, take_over};

/// Config backup key of `HKCU\Control Panel\Keyboard\PrintScreenKeyForSnippingEnabled`.
const BACKUP_KEY: &str = "win:PrintScreenKeyForSnippingEnabled";

/// Modifiers held: Ctrl, Alt, Shift, Win.
type Mods = [bool; 4];

/// A combo as the hook sees it: virtual key, modifiers, and what it runs.
type Binding = (u32, Mods, Cmd);

/// The combos rshot owns: the configured ones, then PrtScn → area (hard-wired while taken over).
/// An empty shortcut is unbound. A bare key other than PrtScn is refused: the hook would take it
/// from every app.
fn bindings(s: &Shortcuts) -> Result<Vec<Binding>, String> {
    let mut out = vec![];
    for (text, cmd) in [
        (&s.area, Cmd::CaptureArea),
        (&s.screen, Cmd::CaptureScreen),
        (&s.window, Cmd::CaptureWindow),
        (&s.record, Cmd::Record),
    ] {
        if text.is_empty() {
            continue;
        }
        let c = combo::parse(text)
            .ok_or_else(|| format!("\"{text}\" isn't a shortcut rshot can take."))?;
        let mods = [c.ctrl, c.alt, c.shift, c.sup];
        if mods == [false; 4] && c.key != combo::Key::Print {
            return Err(format!(
                "\"{text}\" needs Ctrl, Alt, Shift or Super: alone, rshot would take that key from every app."
            ));
        }
        out.push((combo::vk(c.key), mods, cmd));
    }
    out.push((combo::vk(combo::Key::Print), [false; 4], Cmd::CaptureArea));
    Ok(out)
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Pass,
    Swallow,
    Run(Cmd),
}

struct Hook {
    bindings: Vec<Binding>,
    /// Taken over: set by `take_over`, cleared by `release`.
    enabled: bool,
    /// Settings is recording a new shortcut, so the keys must reach it.
    paused: bool,
    /// Keys whose press rshot took, by virtual key.
    held: [bool; 256],
}

impl Hook {
    const fn new() -> Self {
        Self {
            bindings: Vec::new(),
            enabled: false,
            paused: false,
            held: [false; 256],
        }
    }

    /// One key event. A key whose press rshot took has its auto-repeats and its release
    /// swallowed too, so a held PrtScn captures once and no app gets a release without a press.
    // ponytail: a release Windows never delivers (e.g. the secure desktop took it) costs the
    // key's next press, swallowed as a repeat; check GetAsyncKeyState(vk) if that shows up.
    fn judge(&mut self, vk: u32, down: bool, mods: Mods) -> Verdict {
        let Some(held) = self.held.get_mut(vk as usize) else {
            return Verdict::Pass;
        };
        if !down {
            return if std::mem::take(held) {
                Verdict::Swallow
            } else {
                Verdict::Pass
            };
        }
        if *held {
            return Verdict::Swallow;
        }
        if !self.enabled || self.paused {
            return Verdict::Pass;
        }
        match self.bindings.iter().find(|b| b.0 == vk && b.1 == mods) {
            Some(&(_, _, cmd)) => {
                *held = true;
                Verdict::Run(cmd)
            }
            None => Verdict::Pass,
        }
    }
}

/// The registry value as backed up: its number, or "" when it wasn't set.
fn saved(v: Option<u32>) -> String {
    v.map(|v| v.to_string()).unwrap_or_default()
}

/// What a backup puts back: the number, or `None` to delete the value (Windows' default).
fn original(saved: &str) -> Option<u32> {
    saved.parse().ok()
}

#[cfg(target_os = "windows")]
mod sys {
    use super::{bindings, original, saved, Hook, Verdict, BACKUP_KEY};
    use crate::{
        cli::Cmd,
        store::{self, Config},
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex, MutexGuard, OnceLock, PoisonError,
    };
    use tauri::AppHandle;
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, LPARAM, LRESULT, WPARAM},
            System::Registry::{
                RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD,
                RRF_RT_REG_DWORD,
            },
            UI::{
                Input::KeyboardAndMouse::{
                    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU,
                    VK_RWIN, VK_SHIFT,
                },
                WindowsAndMessaging::{
                    CallNextHookEx, GetMessageW, SetWindowsHookExW, HC_ACTION, KBDLLHOOKSTRUCT,
                    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
                },
            },
        },
    };

    static HOOK: Mutex<Hook> = Mutex::new(Hook::new());
    /// Whether `SetWindowsHookExW` succeeded: without the hook, taking the keys would kill them.
    static HOOKED: AtomicBool = AtomicBool::new(false);
    static SENDER: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();

    const SUBKEY: PCWSTR = w!("Control Panel\\Keyboard");
    const VALUE: PCWSTR = w!("PrintScreenKeyForSnippingEnabled");

    /// Never panics: the hook runs it, and a panic there aborts rshot.
    fn hook_state() -> MutexGuard<'static, Hook> {
        HOOK.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn read_snipping() -> Result<Option<u32>, String> {
        let mut v = 0u32;
        let mut size = 4u32;
        // SAFETY: `v` and `size` outlive the call, and RRF_RT_REG_DWORD writes at most 4 bytes.
        let e = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                SUBKEY,
                VALUE,
                RRF_RT_REG_DWORD,
                None,
                Some(&mut v as *mut u32 as *mut _),
                Some(&mut size),
            )
        };
        match e {
            ERROR_FILE_NOT_FOUND => Ok(None),
            e => e
                .ok()
                .map(|()| Some(v))
                .map_err(|e| format!("Reading PrintScreenKeyForSnippingEnabled: {e}")),
        }
    }

    /// Sets the value, or deletes it for `None` (deleting what isn't there is done already).
    fn write_snipping(v: Option<u32>) -> Result<(), String> {
        // SAFETY: plain registry calls; `v` outlives RegSetKeyValueW, which reads 4 bytes of it.
        let e = match v {
            Some(v) => unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    SUBKEY,
                    VALUE,
                    REG_DWORD.0,
                    Some(&v as *const u32 as *const _),
                    4,
                )
            },
            None => match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, SUBKEY, VALUE) } {
                ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                e => e,
            },
        };
        e.ok()
            .map_err(|e| format!("Setting PrintScreenKeyForSnippingEnabled: {e}"))
    }

    /// Refuses, touching nothing, when a shortcut can't be taken or the hook isn't in.
    pub fn take_over(cfg: &mut Config) -> Result<(), String> {
        let b = bindings(&cfg.shortcuts)?;
        if !HOOKED.load(Ordering::SeqCst) {
            return Err(
                "rshot couldn't install its keyboard hook, so it can't take the shortcuts.".into(),
            );
        }
        // Backed up once: a later takeover must not save rshot's own 0 as the original.
        if !cfg
            .gnome_backup
            .as_ref()
            .is_some_and(|b| b.contains_key(BACKUP_KEY))
        {
            let v = saved(read_snipping()?);
            cfg.gnome_backup
                .get_or_insert_with(Default::default)
                .insert(BACKUP_KEY.into(), v);
            // The original reaches disk before the first change, so a crash can still restore it.
            store::save_config(cfg).map_err(|e| format!("saving shortcut backup: {e}"))?;
        }
        write_snipping(Some(0))?;
        let mut h = hook_state();
        h.bindings = b;
        h.enabled = true;
        Ok(())
    }

    /// Hook off and the original value back; the backup (and the consent) stay.
    pub fn release(cfg: &Config) -> Result<(), String> {
        hook_state().enabled = false;
        match cfg.gnome_backup.as_ref().and_then(|b| b.get(BACKUP_KEY)) {
            Some(v) => write_snipping(original(v)),
            None => Ok(()),
        }
    }

    pub fn restore(cfg: &mut Config) -> Result<(), String> {
        release(cfg)?;
        if let Some(b) = &mut cfg.gnome_backup {
            b.remove(BACKUP_KEY);
            if b.is_empty() {
                cfg.gnome_backup = None;
            }
        }
        Ok(())
    }

    pub fn pause(on: bool) {
        hook_state().paused = on;
    }

    fn is_down(vk: VIRTUAL_KEY) -> bool {
        // SAFETY: no pointers involved.
        unsafe { GetAsyncKeyState(i32::from(vk.0)) < 0 }
    }

    /// A dummy key (0xE8 is unassigned) after a swallowed Win or Alt combo, so releasing Win
    /// doesn't open Start, nor Alt the focused app's menu (Alt+PrtScn is the window capture).
    fn mask_modifier() {
        let key = |flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0xE8),
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        // SAFETY: the INPUTs are fully initialised keyboard events.
        unsafe {
            SendInput(
                &[key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)],
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            // SAFETY: for HC_ACTION, lparam points to this event's KBDLLHOOKSTRUCT.
            let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let msg = wparam.0 as u32;
            let mods = [
                is_down(VK_CONTROL),
                is_down(VK_MENU),
                is_down(VK_SHIFT),
                is_down(VK_LWIN) || is_down(VK_RWIN),
            ];
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let verdict = hook_state().judge(kb.vkCode, down, mods);
            match verdict {
                Verdict::Pass => {}
                Verdict::Swallow => return LRESULT(1),
                Verdict::Run(cmd) => {
                    if mods[1] || mods[3] {
                        mask_modifier();
                    }
                    if let Some(tx) = SENDER.get() {
                        let _ = tx.send(cmd);
                    }
                    return LRESULT(1);
                }
            }
        }
        // SAFETY: passes the event on unchanged.
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    /// Starts the hook's thread (a hook needs a message loop) and a worker that runs the commands
    /// off it (Windows drops a slow hook). Returns once the hook is in or has failed.
    pub fn install_hook(app: AppHandle) {
        let (tx, rx) = mpsc::channel::<Cmd>();
        if SENDER.set(tx).is_err() {
            return; // already installed
        }
        std::thread::spawn(move || {
            for cmd in rx {
                crate::dispatch(&app, cmd);
            }
        });
        let (hooked_tx, hooked_rx) = mpsc::channel();
        std::thread::spawn(move || {
            // SAFETY: `hook` is a valid HOOKPROC for the life of the process.
            let hooked = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0) }.is_ok();
            let _ = hooked_tx.send(hooked);
            let mut msg = MSG::default();
            // SAFETY: `msg` outlives each call. 0 = WM_QUIT, -1 = error: stop either way.
            while hooked && unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {}
        });
        HOOKED.store(hooked_rx.recv().unwrap_or(false), Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = [false; 4];
    const WIN_SHIFT: Mods = [false, false, true, true];
    const PRINT: u32 = 0x2C;
    const S: u32 = 0x53;

    fn shortcuts(area: &str, screen: &str, window: &str, record: &str) -> Shortcuts {
        Shortcuts {
            area: area.into(),
            screen: screen.into(),
            window: window.into(),
            record: record.into(),
        }
    }

    #[test]
    fn windows_defaults_bind_with_prtscn_hard_wired() {
        let b = bindings(&shortcuts(
            "Super+Shift+S",
            "Super+Print",
            "Alt+Print",
            "Super+Shift+R",
        ))
        .unwrap();
        assert_eq!(
            b,
            [
                (S, WIN_SHIFT, Cmd::CaptureArea),
                (PRINT, [false, false, false, true], Cmd::CaptureScreen),
                (PRINT, [false, true, false, false], Cmd::CaptureWindow),
                (0x52, WIN_SHIFT, Cmd::Record),
                (PRINT, NONE, Cmd::CaptureArea),
            ]
        );
    }

    #[test]
    fn empty_is_unbound_but_bad_and_bare_combos_are_refused() {
        let b = bindings(&shortcuts("Ctrl+Alt+A", "", "", "")).unwrap();
        assert_eq!(
            b,
            [
                (0x41, [true, true, false, false], Cmd::CaptureArea),
                (PRINT, NONE, Cmd::CaptureArea),
            ]
        );
        assert!(bindings(&shortcuts("Print", "", "", "")).is_ok());
        let e = bindings(&shortcuts("Ctrl+Space", "", "", "")).unwrap_err();
        assert!(e.contains("\"Ctrl+Space\""), "{e}");
        for bare in ["F12", "A", "4"] {
            let e = bindings(&shortcuts("", "", bare, "")).unwrap_err();
            assert!(e.contains(&format!("\"{bare}\"")), "{e}");
        }
    }

    fn on() -> Hook {
        Hook {
            bindings: bindings(&shortcuts("Super+Shift+S", "", "Alt+Print", "")).unwrap(),
            enabled: true,
            ..Hook::new()
        }
    }

    #[test]
    fn a_taken_key_runs_once_and_its_repeats_and_release_are_swallowed() {
        let mut h = on();
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Run(Cmd::CaptureArea));
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Swallow); // auto-repeat
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Swallow);
        assert_eq!(h.judge(PRINT, false, NONE), Verdict::Swallow); // its release
        assert_eq!(h.judge(PRINT, false, NONE), Verdict::Pass); // cleared
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Run(Cmd::CaptureArea));
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let mut h = on();
        assert_eq!(h.judge(S, true, WIN_SHIFT), Verdict::Run(Cmd::CaptureArea));
        assert_eq!(h.judge(S, false, NONE), Verdict::Swallow); // Win/Shift let go first
        assert_eq!(h.judge(S, true, [false, false, true, false]), Verdict::Pass);
        assert_eq!(h.judge(S, false, NONE), Verdict::Pass);
        assert_eq!(h.judge(S, true, [true, false, true, true]), Verdict::Pass);
        assert_eq!(
            h.judge(PRINT, true, [false, true, false, false]),
            Verdict::Run(Cmd::CaptureWindow)
        );
        assert_eq!(h.judge(0x41, true, NONE), Verdict::Pass);
        assert_eq!(h.judge(0x1_0000, true, NONE), Verdict::Pass); // out of range
    }

    #[test]
    fn off_or_paused_passes_new_presses_but_finishes_a_taken_one() {
        let mut h = Hook::new();
        h.bindings = on().bindings;
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Pass); // not taken over
        assert_eq!(h.judge(PRINT, false, NONE), Verdict::Pass);
        let mut h = on();
        h.paused = true; // Settings is recording a shortcut
        assert_eq!(h.judge(S, true, WIN_SHIFT), Verdict::Pass);
        assert_eq!(h.judge(S, false, WIN_SHIFT), Verdict::Pass);
        h.paused = false;
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Run(Cmd::CaptureArea));
        h.enabled = false; // released while PrtScn is held
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Swallow);
        assert_eq!(h.judge(PRINT, false, NONE), Verdict::Swallow);
        assert_eq!(h.judge(PRINT, true, NONE), Verdict::Pass);
    }

    #[test]
    fn a_backup_puts_back_the_value_or_deletes_it() {
        assert_eq!(saved(Some(1)), "1");
        assert_eq!(saved(Some(0)), "0");
        assert_eq!(saved(None), "");
        assert_eq!(original(&saved(Some(1))), Some(1));
        assert_eq!(original(&saved(Some(0))), Some(0));
        assert_eq!(original(&saved(None)), None);
        assert_eq!(original("junk"), None); // hand-edited: Windows' default
    }
}
