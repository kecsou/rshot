//! macOS: switch off the system screenshot hotkeys (symbolic hotkeys), then own ⌘⇧3/4/5 (and the ⌃
//! variants) as global shortcuts. Every decision is a plain function, unit-tested on every OS;
//! only `sys` calls macOS.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::{
    cli::Cmd,
    combo::{self, Combo, Key},
    store::{Config, Shortcuts},
};

#[cfg(target_os = "macos")]
pub use sys::{init, release, restore, take_over};

/// (symbolic hotkey id, ASCII, virtual key code, modifier mask) of ⌘⇧3, ⌃⌘⇧3, ⌘⇧4, ⌃⌘⇧4, ⌘⇧5:
/// the system's own parameters, written back with each enabled bit.
const SYSTEM: [(u32, u32, u32, u32); 5] = [
    (28, 51, 20, 1179648),
    (29, 51, 20, 1441792),
    (30, 52, 21, 1179648),
    (31, 52, 21, 1441792),
    (184, 53, 23, 1179648),
];

/// Config backup key of hotkey `id`'s enabled bit ("1"/"0").
fn backup_key(id: u32) -> String {
    format!("mac:{id}")
}

/// The value `defaults write com.apple.symbolichotkeys AppleSymbolicHotKeys -dict-add <id>` gets:
/// the hotkey with the system's parameters, typed as System Settings writes them.
fn entry(enabled: bool, (_, ascii, vk, mods): (u32, u32, u32, u32)) -> String {
    format!(
        "<dict><key>enabled</key><{enabled}/><key>value</key><dict><key>parameters</key>\
         <array><integer>{ascii}</integer><integer>{vk}</integer><integer>{mods}</integer></array>\
         <key>type</key><string>standard</string></dict></dict>"
    )
}

/// A hotkey's state from `plutil -extract AppleSymbolicHotKeys.<id>.enabled raw`; `None` (no
/// entry: never changed) is macOS's default, on.
fn is_enabled(plutil: Option<&str>) -> bool {
    !matches!(plutil.map(str::trim), Some("false" | "0"))
}

/// What the backup holds before rshot switches the hotkey off: its state now, which is the user's
/// choice (even one made after a Quit gave it back), unless it is off while a backup exists:
/// rshot's own off, left by this or a crashed session, so the backup stays.
fn backup(kept: Option<&str>, now_enabled: bool) -> String {
    match kept {
        Some(kept) if !now_enabled => kept.to_string(),
        _ => u8::from(now_enabled).to_string(),
    }
}

/// The hotkeys a release puts back, with their backed-up state: only those rshot backed up (so
/// switched off); "0" stays off, anything else (even hand-edited junk) is macOS's default, on.
fn originals(cfg: &Config) -> Vec<(u32, bool)> {
    let Some(b) = &cfg.gnome_backup else {
        return vec![];
    };
    SYSTEM
        .iter()
        .filter_map(|&(id, ..)| b.get(&backup_key(id)).map(|v| (id, v != "0")))
        .collect()
}

/// The combos rshot owns: the configured ones (⌘⇧3 and ⌘⇧4 also with ⌃, as macOS has them), then
/// ⌘⇧5 → area (hard-wired while taken over). An empty shortcut is unbound; a bad one is refused
/// (`combo::bindable`). A combo bound twice keeps its first action: macOS refuses to register it
/// again.
fn bindings(s: &Shortcuts) -> Result<Vec<(Combo, Cmd)>, String> {
    fn add(out: &mut Vec<(Combo, Cmd)>, c: Combo, cmd: Cmd) {
        if !out.iter().any(|(b, _)| *b == c) {
            out.push((c, cmd));
        }
    }
    let cmd_shift = |key| Combo {
        ctrl: false,
        alt: false,
        shift: true,
        sup: true,
        key,
    };
    let mut out = vec![];
    for (text, cmd) in [
        (&s.area, Cmd::CaptureArea),
        (&s.screen, Cmd::CaptureScreen),
        (&s.window, Cmd::CaptureWindow),
        (&s.record, Cmd::Record),
    ] {
        if let Some(c) = combo::bindable(text)? {
            add(&mut out, c, cmd);
            if c == cmd_shift(Key::Digit(3)) || c == cmd_shift(Key::Digit(4)) {
                add(&mut out, Combo { ctrl: true, ..c }, cmd);
            }
        }
    }
    add(&mut out, cmd_shift(Key::Digit(5)), Cmd::CaptureArea);
    Ok(out)
}

/// macOS key codes of the typing keys, named by the `Code` global-hotkey registers for each (its
/// `key_to_scancode`: US-ANSI positions).
const KEYS: [(u16, &str); 47] = [
    (0x00, "KeyA"),
    (0x01, "KeyS"),
    (0x02, "KeyD"),
    (0x03, "KeyF"),
    (0x04, "KeyH"),
    (0x05, "KeyG"),
    (0x06, "KeyZ"),
    (0x07, "KeyX"),
    (0x08, "KeyC"),
    (0x09, "KeyV"),
    (0x0b, "KeyB"),
    (0x0c, "KeyQ"),
    (0x0d, "KeyW"),
    (0x0e, "KeyE"),
    (0x0f, "KeyR"),
    (0x10, "KeyY"),
    (0x11, "KeyT"),
    (0x12, "Digit1"),
    (0x13, "Digit2"),
    (0x14, "Digit3"),
    (0x15, "Digit4"),
    (0x16, "Digit6"),
    (0x17, "Digit5"),
    (0x18, "Equal"),
    (0x19, "Digit9"),
    (0x1a, "Digit7"),
    (0x1b, "Minus"),
    (0x1c, "Digit8"),
    (0x1d, "Digit0"),
    (0x1e, "BracketRight"),
    (0x1f, "KeyO"),
    (0x20, "KeyU"),
    (0x21, "BracketLeft"),
    (0x22, "KeyI"),
    (0x23, "KeyP"),
    (0x25, "KeyL"),
    (0x26, "KeyJ"),
    (0x27, "Quote"),
    (0x28, "KeyK"),
    (0x29, "Semicolon"),
    (0x2a, "Backslash"),
    (0x2b, "Comma"),
    (0x2c, "Slash"),
    (0x2d, "KeyN"),
    (0x2e, "KeyM"),
    (0x2f, "Period"),
    (0x32, "Backquote"),
];

/// The `Code` name to register for `k`. global-hotkey binds physical keys, but Settings records the
/// letter the layout types: so a letter is the key that types it in `layout` (each typing key's
/// `Code` name and what it types unshifted), e.g. AZERTY's A is the US Q key; a layout without it
/// gives the US key. Digits and F-keys are physical, as recorded.
fn code_name(k: Key, layout: &[(&str, char)]) -> String {
    match k {
        Key::Print => "PrintScreen".into(),
        Key::Digit(d) => format!("Digit{d}"),
        Key::F(n) => format!("F{n}"),
        Key::Letter(c) => layout
            .iter()
            .find(|(_, typed)| typed.eq_ignore_ascii_case(&c))
            .map_or_else(|| format!("Key{c}"), |(name, _)| (*name).into()),
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use super::{backup, backup_key, bindings, code_name, entry, is_enabled, originals, KEYS};
    use super::{Cmd, Combo, SYSTEM};
    use crate::{
        err, pipeline,
        store::{self, Config},
    };
    use std::{process::Command, sync::OnceLock};
    use tauri::AppHandle;
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    /// The daemon's handle, for the global shortcuts; unset in a CLI `restore-shortcuts`.
    static APP: OnceLock<AppHandle> = OnceLock::new();

    pub fn init(app: AppHandle) {
        let _ = APP.set(app);
    }

    const ACTIVATE: &str =
        "/System/Library/PrivateFrameworks/SystemAdministration.framework/Resources/activateSettings";

    /// Runs a tool to completion: its stdout, or why it failed.
    fn run(tool: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(tool)
            .args(args)
            .output()
            .map_err(|e| format!("{tool}: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(format!(
                "{tool} {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    fn read_enabled(id: u32) -> bool {
        let plist = dirs::home_dir()
            .unwrap_or_default()
            .join("Library/Preferences/com.apple.symbolichotkeys.plist");
        let key = format!("AppleSymbolicHotKeys.{id}.enabled");
        let args = ["-extract", &key, "raw", "-o", "-", &plist.to_string_lossy()];
        is_enabled(run("/usr/bin/plutil", &args).ok().as_deref())
    }

    /// Writes the enabled bit of each listed hotkey, then makes the change live.
    fn set_system(bits: &[(u32, bool)]) -> Result<(), String> {
        for &(id, on) in bits {
            let Some(&hotkey) = SYSTEM.iter().find(|s| s.0 == id) else {
                continue;
            };
            let args = [
                "write",
                "com.apple.symbolichotkeys",
                "AppleSymbolicHotKeys",
                "-dict-add",
                &id.to_string(),
                &entry(on, hotkey),
            ];
            run("/usr/bin/defaults", &args)?;
        }
        if !bits.is_empty() {
            run(ACTIVATE, &["-u"])?;
        }
        Ok(())
    }

    /// Refuses, touching nothing, when a shortcut can't be taken. The system's hotkeys go off here;
    /// the global shortcuts are registered on the main thread just after (a failure there puts the
    /// system's back and says so).
    pub fn take_over(cfg: &mut Config) -> Result<(), String> {
        let wanted = bindings(&cfg.shortcuts)?;
        let app = APP.get().ok_or("rshot isn't running")?.clone();
        let kept = cfg.gnome_backup.get_or_insert_with(Default::default);
        let mut changed = false;
        for (id, ..) in SYSTEM {
            let k = backup_key(id);
            let v = backup(kept.get(&k).map(String::as_str), read_enabled(id));
            if kept.get(&k) != Some(&v) {
                kept.insert(k, v);
                changed = true;
            }
        }
        if changed {
            // The originals reach disk before the first change, so a crash can still restore them.
            store::save_config(cfg).map_err(|e| format!("saving shortcut backup: {e}"))?;
        }
        set_system(&SYSTEM.map(|s| (s.0, false)))?;
        let back = originals(cfg);
        // Registering waits for the main thread, and the caller holds `config`: posted, never
        // waited for (main.rs lock rule). From setup, on the main thread, it runs right here.
        let a = app.clone();
        app.run_on_main_thread(move || {
            if let Err(e) = register(&a, &wanted) {
                // Half a set is no use: the system's own hotkeys come back instead.
                let _ = a.global_shortcut().unregister_all();
                pipeline::notify(
                    &a,
                    &format!("rshot couldn't take the screenshot shortcuts ({e}); the system's are back."),
                );
                std::thread::spawn(move || {
                    if let Err(e) = set_system(&back) {
                        pipeline::notify(&a, &format!("Putting the system's shortcuts back: {e}"));
                    }
                });
            }
        })
        .map_err(err)
    }

    /// Main thread only: Text Input Sources (the layout) require it, and the plugin would wait on it.
    fn register(app: &AppHandle, wanted: &[(Combo, Cmd)]) -> Result<(), String> {
        let gs = app.global_shortcut();
        gs.unregister_all().map_err(err)?;
        let layout = layout();
        for &(c, cmd) in wanted {
            let name = code_name(c.key, &layout);
            let code: Code = name.parse().map_err(|_| format!("no key {name}"))?;
            let mut m = Modifiers::empty();
            m.set(Modifiers::CONTROL, c.ctrl);
            m.set(Modifiers::ALT, c.alt);
            m.set(Modifiers::SHIFT, c.shift);
            m.set(Modifiers::SUPER, c.sup);
            gs.on_shortcut(Shortcut::new(Some(m), code), move |app, _, e| {
                if e.state == ShortcutState::Pressed {
                    // Off the main thread, where macOS delivers hotkeys: a capture waits for
                    // rshot's own UI to leave the screen, which needs the main thread.
                    let app = app.clone();
                    std::thread::spawn(move || crate::dispatch(&app, cmd));
                }
            })
            .map_err(err)?;
        }
        Ok(())
    }

    /// Global shortcuts off and the system's hotkeys back as backed up; the backup (and the
    /// consent) stay.
    pub fn release(cfg: &Config) -> Result<(), String> {
        if let Some(app) = APP.get() {
            let a = app.clone();
            // Posted, not waited for (the caller holds `config`). Posting fails only once the
            // event loop is gone, and the shortcuts with it.
            let _ = app.run_on_main_thread(move || {
                if let Err(e) = a.global_shortcut().unregister_all() {
                    pipeline::notify(&a, &format!("Releasing rshot's shortcuts: {e}"));
                }
            });
        }
        set_system(&originals(cfg))
    }

    pub fn restore(cfg: &mut Config) -> Result<(), String> {
        release(cfg)?;
        if let Some(b) = &mut cfg.gnome_backup {
            b.retain(|k, _| !k.starts_with("mac:"));
            if b.is_empty() {
                cfg.gnome_backup = None;
            }
        }
        Ok(())
    }

    /// What each typing key types in the current keyboard layout, unshifted, by `Code` name; empty
    /// when the layout has no Unicode key data. Main thread only (Text Input Sources).
    // ponytail: read at takeover; a layout switched later keeps the old letter keys until the next
    // takeover (launch or rebind). Upgrade: re-register on kTISNotifySelectedKeyboardInputSourceChanged.
    fn layout() -> Vec<(&'static str, char)> {
        use std::ffi::c_void;
        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
            fn TISGetInputSourceProperty(
                source: *const c_void,
                key: *const c_void,
            ) -> *const c_void;
            #[link_name = "kTISPropertyUnicodeKeyLayoutData"]
            static UNICODE_KEY_LAYOUT_DATA: *const c_void;
            fn LMGetKbdType() -> u8;
            #[allow(clippy::too_many_arguments)]
            fn UCKeyTranslate(
                layout: *const c_void,
                key_code: u16,
                action: u16,
                modifiers: u32,
                keyboard_type: u32,
                options: u32,
                dead_key_state: *mut u32,
                max_len: usize,
                len: *mut usize,
                chars: *mut u16,
            ) -> i32;
        }
        #[link(name = "CoreFoundation", kind = "framework")]
        extern "C" {
            fn CFDataGetBytePtr(data: *const c_void) -> *const c_void;
            fn CFRelease(cf: *const c_void);
        }
        const DISPLAY: u16 = 3; // kUCKeyActionDisplay
        const NO_DEAD_KEYS: u32 = 1; // kUCKeyTranslateNoDeadKeysMask
        let mut out = vec![];
        // SAFETY: the calls as documented (and as tao makes them): the source is released once,
        // its layout data is read only while the source is held, and the buffers outlive each call.
        unsafe {
            let source = TISCopyCurrentKeyboardLayoutInputSource();
            if source.is_null() {
                return out;
            }
            let data = TISGetInputSourceProperty(source, UNICODE_KEY_LAYOUT_DATA);
            if !data.is_null() {
                let layout = CFDataGetBytePtr(data);
                let kind = u32::from(LMGetKbdType());
                for (code, name) in KEYS {
                    let (mut dead, mut len, mut buf) = (0u32, 0usize, [0u16; 4]);
                    let r = UCKeyTranslate(
                        layout,
                        code,
                        DISPLAY,
                        0,
                        kind,
                        NO_DEAD_KEYS,
                        &mut dead,
                        buf.len(),
                        &mut len,
                        buf.as_mut_ptr(),
                    );
                    let typed = buf.get(..len).filter(|_| r == 0);
                    if let Some(Ok(c)) =
                        typed.and_then(|t| char::decode_utf16(t.iter().copied()).next())
                    {
                        out.push((name, c));
                    }
                }
            }
            CFRelease(source);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortcuts(area: &str, screen: &str, window: &str, record: &str) -> Shortcuts {
        Shortcuts {
            area: area.into(),
            screen: screen.into(),
            window: window.into(),
            record: record.into(),
        }
    }

    fn c(s: &str) -> Combo {
        combo::parse(s).unwrap()
    }

    #[test]
    fn macos_defaults_bind_with_ctrl_variants_and_cmd_shift_5() {
        let b = bindings(&shortcuts("Super+Shift+4", "Super+Shift+3", "", "")).unwrap();
        assert_eq!(
            b,
            [
                (c("Super+Shift+4"), Cmd::CaptureArea),
                (c("Ctrl+Super+Shift+4"), Cmd::CaptureArea),
                (c("Super+Shift+3"), Cmd::CaptureScreen),
                (c("Ctrl+Super+Shift+3"), Cmd::CaptureScreen),
                (c("Super+Shift+5"), Cmd::CaptureArea),
            ]
        );
    }

    #[test]
    fn ctrl_variants_only_for_cmd_shift_3_and_4_and_the_first_binding_wins() {
        let b = bindings(&shortcuts(
            "Super+Shift+5",
            "Super+Shift+6",
            "Alt+Super+4",
            "Ctrl+Alt+R",
        ))
        .unwrap();
        assert_eq!(
            b,
            [
                (c("Super+Shift+5"), Cmd::CaptureArea),
                (c("Super+Shift+6"), Cmd::CaptureScreen),
                (c("Alt+Super+4"), Cmd::CaptureWindow),
                (c("Ctrl+Alt+R"), Cmd::Record),
            ]
        );
        // ⌃⌘⇧3 taken by the window capture first: the screen's ⌃ variant doesn't take it again.
        let b = bindings(&shortcuts("", "Super+Shift+3", "Ctrl+Super+Shift+3", "")).unwrap();
        assert_eq!(b[1], (c("Ctrl+Super+Shift+3"), Cmd::CaptureScreen));
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn bad_and_bare_combos_are_refused() {
        let e = bindings(&shortcuts("Super+Space", "", "", "")).unwrap_err();
        assert!(e.contains("\"Super+Space\""), "{e}");
        let e = bindings(&shortcuts("", "Shift+3", "", "")).unwrap_err();
        assert!(e.contains("\"Shift+3\""), "{e}");
    }

    #[test]
    fn letters_follow_the_layout_digits_and_f_keys_stay_physical() {
        // AZERTY: the US Q key types a, A types q, W types z, Z types w, the US ; key types m.
        let azerty = [
            ("KeyQ", 'a'),
            ("KeyA", 'q'),
            ("KeyW", 'z'),
            ("KeyZ", 'w'),
            ("Semicolon", 'm'),
            ("KeyM", ','),
            ("Digit3", '"'),
        ];
        assert_eq!(code_name(Key::Letter('A'), &azerty), "KeyQ");
        assert_eq!(code_name(Key::Letter('Q'), &azerty), "KeyA");
        assert_eq!(code_name(Key::Letter('M'), &azerty), "Semicolon");
        assert_eq!(code_name(Key::Letter('R'), &azerty), "KeyR"); // not listed: the US key
        assert_eq!(code_name(Key::Letter('A'), &[]), "KeyA"); // no layout data
        assert_eq!(code_name(Key::Digit(3), &azerty), "Digit3");
        assert_eq!(code_name(Key::F(13), &azerty), "F13");
        assert_eq!(code_name(Key::Print, &azerty), "PrintScreen");
        // Every name is one global-hotkey maps, and the table has no duplicates.
        let names: std::collections::BTreeSet<_> = KEYS.iter().map(|k| k.1).collect();
        assert_eq!(names.len(), KEYS.len());
        assert!(('A'..='Z').all(|l| names.contains(format!("Key{l}").as_str())));
    }

    #[test]
    fn entries_carry_the_bit_and_the_system_parameters() {
        assert_eq!(
            entry(false, SYSTEM[0]),
            "<dict><key>enabled</key><false/><key>value</key><dict><key>parameters</key>\
             <array><integer>51</integer><integer>20</integer><integer>1179648</integer></array>\
             <key>type</key><string>standard</string></dict></dict>"
        );
        assert!(entry(true, SYSTEM[4]).starts_with("<dict><key>enabled</key><true/>"));
        assert!(entry(true, SYSTEM[4]).contains("<integer>53</integer><integer>23</integer>"));
    }

    #[test]
    fn plutil_output_reads_as_on_unless_off() {
        assert!(is_enabled(None)); // no entry: never changed
        assert!(is_enabled(Some("true\n")));
        assert!(is_enabled(Some("1")));
        assert!(!is_enabled(Some("false\n")));
        assert!(!is_enabled(Some("0"))); // written as an old-style string
    }

    #[test]
    fn the_backup_follows_the_users_state_but_never_takes_rshots_off() {
        assert_eq!(backup(None, true), "1"); // first takeover
        assert_eq!(backup(None, false), "0"); // the user's own off
        assert_eq!(backup(Some("1"), false), "1"); // rshot's off (still taken, or a crash)
        assert_eq!(backup(Some("0"), false), "0");
        // Changed by the user while rshot wasn't holding it (after a Quit): theirs now.
        assert_eq!(backup(Some("0"), true), "1");
    }

    #[test]
    fn a_release_puts_back_only_what_was_backed_up() {
        let mut cfg = Config::default();
        assert_eq!(originals(&cfg), []);
        cfg.gnome_backup = Some(
            [
                ("mac:28", "1"),
                ("mac:29", "0"),
                ("mac:184", "junk"),
                ("win:X", "0"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .into(),
        );
        assert_eq!(originals(&cfg), [(28, true), (29, false), (184, true)]);
    }
}
