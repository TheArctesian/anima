//! The characters that a person does not see, or that reorder what the person reads, in a text
//! of a program: a request, a rule or a project file (THR-125).

/// Whether `each` is a mark of direction, a mark with no width, a separator of lines or
/// paragraphs that is not a new line, or a tag: a character that can make a text look different
/// from what it is. The control characters are not in it; each caller decides for those.
#[must_use]
pub const fn is_hidden_mark(each: char) -> bool {
    matches!(
        each,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

#[cfg(test)]
mod tests {
    use super::is_hidden_mark;

    #[test]
    fn thr_125_the_marks_that_hide_or_reorder_text_are_hidden_and_letters_are_not() {
        for mark in [
            '\u{061C}',
            '\u{200B}',
            '\u{200D}',
            '\u{2028}',
            '\u{2029}',
            '\u{202E}',
            '\u{2066}',
            '\u{FEFF}',
            '\u{00AD}',
            '\u{E0041}',
        ] {
            assert!(is_hidden_mark(mark), "{:X}", u32::from(mark));
        }
        for shown in ['a', 'é', 'ש', '漢', ' ', '…'] {
            assert!(!is_hidden_mark(shown));
        }
    }
}
