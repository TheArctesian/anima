//! What counts as plain text: no control character and nothing that hides, reorders or
//! breaks text when it is shown (Q2-11). Each domain decides what to do with a bad character.

/// Format characters (Unicode Cf) and the line and paragraph separators (Zl, Zp): bidi
/// controls, zero-width characters, the byte order mark, tag characters and the like.
#[must_use]
pub fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{ad}' | '\u{600}'..='\u{605}' | '\u{61c}' | '\u{6dd}' | '\u{70f}'
        | '\u{890}'..='\u{891}' | '\u{8e2}' | '\u{180e}'
        | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}'
        | '\u{feff}' | '\u{fff9}'..='\u{fffb}'
        | '\u{110bd}' | '\u{110cd}' | '\u{13430}'..='\u{1343f}' | '\u{1bca0}'..='\u{1bca3}'
        | '\u{1d173}'..='\u{1d17a}' | '\u{e0000}'..='\u{e007f}')
}

/// Whether the text has no control character and no invisible character.
#[must_use]
pub fn is_plain(s: &str) -> bool {
    !s.chars().any(|c| c.is_control() || is_invisible(c))
}

/// The text without control or invisible characters, at most `max_chars` characters.
#[must_use]
pub fn clean(s: &str, max_chars: usize) -> String {
    s.chars()
        .filter(|c| !c.is_control() && !is_invisible(*c))
        .take(max_chars)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{clean, is_invisible, is_plain};

    fn all_invisible(chars: impl IntoIterator<Item = char>) -> bool {
        chars.into_iter().all(is_invisible)
    }

    #[test]
    fn q2_11_bidi_controls_are_invisible() {
        assert!(all_invisible([
            '\u{61c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}'
        ]));
    }

    #[test]
    fn q2_11_zero_width_characters_are_invisible() {
        assert!(all_invisible([
            '\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}', '\u{ad}'
        ]));
    }

    #[test]
    fn q2_11_line_and_paragraph_separators_are_invisible() {
        assert!(all_invisible(['\u{2028}', '\u{2029}']));
    }

    #[test]
    fn q2_11_tag_characters_are_invisible() {
        assert!(all_invisible(['\u{e0001}', '\u{e0020}', '\u{e007f}']));
    }

    #[test]
    fn q2_11_the_widest_table_keeps_the_sessions_additions() {
        assert!(all_invisible([
            '\u{890}',
            '\u{8e2}',
            '\u{110bd}',
            '\u{1d173}'
        ]));
    }

    #[test]
    fn q2_11_plain_text_passes_and_bad_characters_do_not() {
        assert!(is_plain("Küche \u{4e2d} 12:30, a-b_c"));
        assert!(!is_plain("a\nb"));
        assert!(!is_plain("a\u{2028}b"));
        assert!(!is_plain("a\u{202e}b"));
    }

    #[test]
    fn q2_11_clean_strips_then_truncates_by_characters_not_bytes() {
        assert_eq!(clean("a\r\nb\u{200b}c\u{1b}d", 10), "abcd");
        assert_eq!(
            clean("\u{e4}\u{4e2d}\u{e4}\u{4e2d}", 3),
            "\u{e4}\u{4e2d}\u{e4}"
        );
    }
}
