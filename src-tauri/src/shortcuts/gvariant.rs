//! The little GVariant text that `gsettings get/set` speaks, plus accelerator conversion.

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

/// "Ctrl+Alt+Shift+R" → "<Ctrl><Alt><Shift>R" (GTK accelerator syntax).
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

/// Adds `ours` to a custom-keybindings list without duplicates, keeping everything else.
pub fn with_paths(existing: &[String], ours: &[String]) -> Vec<String> {
    let mut v = existing.to_vec();
    v.extend(ours.iter().filter(|p| !existing.contains(p)).cloned());
    v
}

pub fn without_paths(existing: &[String], ours: &[String]) -> Vec<String> {
    existing.iter().filter(|p| !ours.contains(p)).cloned().collect()
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
