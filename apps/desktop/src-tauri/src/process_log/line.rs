use std::fmt::Arguments;

use tauri_plugin_log::{TimezoneStrategy, log::Record};

/// A UTC time stamp: (year, month, day) and (hour, minute, second).
pub(super) struct Stamp {
    pub date: (i32, u8, u8),
    pub time: (u8, u8, u8),
}

impl Stamp {
    /// The current UTC time, read from the clock.
    pub(super) fn now() -> Self {
        let now = TimezoneStrategy::UseUtc.get_now();
        Self {
            date: (now.year(), u8::from(now.month()), now.day()),
            time: (now.hour(), now.minute(), now.second()),
        }
    }
}

/// One log line for `record`: `2026-11-04T09:30:00Z [INFO] [target] message`. `message` is the
/// record's text as the logger hands it on (fern's rule: use it, not `record.args()`).
///
/// The web view chooses its own target and message through `log:allow-log`, so control
/// characters, line separators and bidi controls in both are escaped: one record is always
/// one line, and no record can forge or visually reorder another.
pub(super) fn line_of(stamp: &Stamp, record: &Record, message: &Arguments) -> String {
    let (year, month, day) = stamp.date;
    let (hour, minute, second) = stamp.time;
    let level = record.level();
    let mut line =
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z [{level}] [");
    push_escaped(&mut line, record.target());
    line.push_str("] ");
    push_escaped(&mut line, &message.to_string());
    line
}

/// Characters that are written as their Rust escape: C0 and C1 controls (`is_control`), the
/// Unicode line and paragraph separators, and the bidi embedding, override and isolate controls.
fn is_unsafe_in_a_line(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
        )
}

/// Appends `text` with every unsafe character written as its escape (`\n`, `\u{1b}`).
/// Every other character is kept as is.
fn push_escaped(line: &mut String, text: &str) {
    for c in text.chars() {
        if is_unsafe_in_a_line(c) {
            line.extend(c.escape_default());
        } else {
            line.push(c);
        }
    }
}
