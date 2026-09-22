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
