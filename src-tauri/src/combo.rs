//! Neutral shortcut strings ("Ctrl+Alt+Shift+R", "Super+Print") ↔ modifiers + key.
#![allow(dead_code)] // wired up by Tasks 2–3

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

pub fn parse(s: &str) -> Option<Combo> {
    let parts: Vec<&str> = s.split('+').collect();
    let (key, mods) = parts.split_last()?;
    let mut c = Combo {
        ctrl: false,
        alt: false,
        shift: false,
        sup: false,
        key: parse_key(key)?,
    };
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

/// Whether rshot may take the combo system-wide: it needs Ctrl, Alt or Super (Shift alone would
/// take capitals from every app), except PrtScn. Settings' recorder applies the same rule (keys.ts).
pub fn takeable(c: &Combo) -> bool {
    c.ctrl || c.alt || c.sup || c.key == Key::Print
}

fn parse_key(k: &str) -> Option<Key> {
    let mut chars = k.chars();
    match (chars.next(), chars.next()) {
        _ if k == "Print" => Some(Key::Print),
        (Some(ch), None) if ch.is_ascii_uppercase() => Some(Key::Letter(ch)),
        (Some(ch), None) if ch.is_ascii_digit() => Some(Key::Digit(ch as u8 - b'0')),
        (Some('F'), Some(_)) => k[1..]
            .parse()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map(Key::F),
        _ => None,
    }
}

/// Windows virtual-key code.
#[cfg(any(target_os = "windows", test))]
pub fn vk(k: Key) -> u32 {
    match k {
        Key::Print => 0x2C,
        Key::Letter(c) => c as u32,
        Key::Digit(d) => u32::from(b'0' + d),
        Key::F(n) => 0x70 + u32::from(n) - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn c(ctrl: bool, alt: bool, shift: bool, sup: bool, key: Key) -> Combo {
        Combo {
            ctrl,
            alt,
            shift,
            sup,
            key,
        }
    }

    #[test]
    fn parses_neutral_combos() {
        assert_eq!(
            parse("Print"),
            Some(c(false, false, false, false, Key::Print))
        );
        assert_eq!(
            parse("Ctrl+Alt+Shift+R"),
            Some(c(true, true, true, false, Key::Letter('R')))
        );
        assert_eq!(
            parse("Super+Shift+4"),
            Some(c(false, false, true, true, Key::Digit(4)))
        );
        assert_eq!(
            parse("Alt+F12"),
            Some(c(false, true, false, false, Key::F(12)))
        );
    }

    #[test]
    fn rejects_empty_and_unknown() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("Ctrl+"), None);
        assert_eq!(parse("Hyper+X"), None);
        assert_eq!(parse("Ctrl+Space"), None);
    }

    #[test]
    fn takeable_needs_ctrl_alt_or_super_unless_print() {
        let t = |s: &str| takeable(&parse(s).unwrap());
        for ok in [
            "Print",
            "Shift+Print",
            "Ctrl+F9",
            "Alt+A",
            "Super+Shift+S",
            "Ctrl+Shift+4",
        ] {
            assert!(t(ok), "{ok}");
        }
        for bad in ["F9", "A", "4", "Shift+A", "Shift+F12"] {
            assert!(!t(bad), "{bad}");
        }
    }

    #[test]
    fn maps_keys_to_windows_virtual_keys() {
        assert_eq!(
            [
                Key::Print,
                Key::Letter('R'),
                Key::Digit(4),
                Key::F(1),
                Key::F(24)
            ]
            .map(vk),
            [0x2C, 0x52, 0x34, 0x70, 0x87]
        );
    }
}
