//! Text from the hub or a device goes to a terminal only after this: control characters could
//! move the cursor, rewrite earlier lines or set the window title.

/// `text` with every control character (escape sequences, line breaks, tabs) as a space, and
/// the bidirectional controls, which reorder what is shown, removed.
#[must_use]
pub fn clean(text: &str) -> String {
    text.chars()
        .filter(|character| !is_bidirectional(*character))
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

/// `text` cleaned line by line: line breaks stay, every other control character goes.
#[must_use]
pub fn clean_lines(text: &str) -> String {
    text.lines().map(clean).collect::<Vec<_>>().join("\n")
}

/// The marks that reorder text as it is shown: embeddings, overrides, isolates and the
/// left-to-right, right-to-left and Arabic letter marks.
const fn is_bidirectional(character: char) -> bool {
    matches!(
        character,
        '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{061C}'
    )
}

/// `word` as one shell word, in single quotes when it holds anything but plain characters, for a
/// hint the operator can paste.
#[must_use]
pub fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", clean(word).replace('\'', "'\\''"))
    }
}

/// The K1 name of a generated enum value, such as `UP` or `MACOS`.
#[must_use]
pub fn enum_name<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_020_control_characters_never_reach_the_terminal() {
        assert_eq!(clean("a\u{1b}]0;title\u{7}b\nc\td"), "a ]0;title b c d");
        assert_eq!(clean("\u{9b}31m"), " 31m");
        assert_eq!(clean("plain é"), "plain é");
    }

    #[test]
    fn cmd_020_s5_bidirectional_controls_are_removed() {
        assert_eq!(clean("gnp.\u{202E}exe\u{2066}x\u{2069}"), "gnp.exex");
        assert_eq!(clean_lines("a\u{1b}b\nc\u{202E}d"), "a b\ncd");
    }

    #[test]
    fn cmd_026_a_hint_quotes_a_word_with_spaces() {
        assert_eq!(shell_word("ping"), "ping");
        assert_eq!(shell_word("HTTP check"), "'HTTP check'");
        assert_eq!(shell_word("it's"), "'it'\\''s'");
    }
}
