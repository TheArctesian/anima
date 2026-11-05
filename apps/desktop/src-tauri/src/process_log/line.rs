use tauri_plugin_log::log::Level;

/// A UTC time stamp: (year, month, day) and (hour, minute, second).
pub(super) struct Stamp {
    pub date: (i32, u8, u8),
    pub time: (u8, u8, u8),
}

/// One log line: `2026-11-04T09:30:00Z [INFO] [target] message`. The web view chooses its own
/// target and message through `log:allow-log`, so control characters in both are escaped:
/// one record is always one line, and nobody can forge a second one.
pub(super) fn render(stamp: &Stamp, level: Level, target: &str, message: &str) -> String {
    let (year, month, day) = stamp.date;
    let (hour, minute, second) = stamp.time;
    let mut line =
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z [{level}] [");
    push_escaped(&mut line, target);
    line.push_str("] ");
    push_escaped(&mut line, message);
    line
}

/// Appends `text` with every control character and Unicode line or paragraph separator written
/// as its Rust escape (`\n`, `\u{1b}`). Every other character is kept as is.
fn push_escaped(line: &mut String, text: &str) {
    for c in text.chars() {
        if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
            line.extend(c.escape_default());
        } else {
            line.push(c);
        }
    }
}
