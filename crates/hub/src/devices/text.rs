//! Which characters operator-typed text may hold. Names, kinds, tags and user names reach the
//! event log and the audit log, which are kept for a year and cannot be edited (LOG-004).

/// Whether the text has no control character (a line break or NUL would split or break a log
/// line) and no character that changes the reading direction (it would disguise the text).
pub(super) fn is_plain(text: &str) -> bool {
    !text
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'))
}
