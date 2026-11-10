//! The two plain-text forms every channel starts from. Control characters (CR, LF, NUL, DEL,
//! the C1 range) and invisible ones (bidirectional overrides, zero-width marks, the line and
//! paragraph separators; `text::is_invisible`) never reach a channel as they are.

use crate::text::is_invisible;

/// One line: each control character becomes a space and each invisible one is dropped, so
/// `a\r\nb` is `a  b`. Leading and trailing spaces go.
pub(super) fn single_line(text: &str) -> String {
    text.chars()
        .filter(|c| !is_invisible(*c))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Several lines: CR LF and a lone CR become LF, tab and LF stay, every other control
/// character and each invisible one is dropped.
pub(super) fn plain_body(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .map(|c| if c == '\r' { '\n' } else { c })
        .filter(|c| matches!(c, '\n' | '\t') || !(c.is_control() || is_invisible(*c)))
        .collect()
}
