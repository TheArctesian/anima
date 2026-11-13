//! Text from the hub or a device goes to a terminal only after this: control characters could
//! move the cursor, rewrite earlier lines or set the window title.

/// `text` with every control character (escape sequences, line breaks, tabs) as a space.
#[must_use]
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
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
}
