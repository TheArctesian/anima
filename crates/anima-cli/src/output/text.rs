//! Text from the hub or a device goes to a terminal or an agent only after this: control
//! characters could move the cursor, rewrite earlier lines or set the window title, and
//! invisible characters could carry text that a person cannot see but an agent reads.

/// `text` with every control character (escape sequences, line breaks, tabs) and the line and
/// paragraph separators as a space; the invisible characters (format characters, private use,
/// noncharacters) removed. Ordinary non-ASCII text stays.
#[must_use]
pub fn clean(text: &str) -> String {
    text.chars()
        .filter(|character| !is_invisible(*character))
        .map(|character| {
            if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
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

/// Whether `character` shows nothing a person can see, but can change what is shown or carry
/// hidden text: a format character (Unicode category Cf), a private-use code point or a
/// noncharacter. Cf is written out from Unicode 16 because the standard library has no table
/// for it; a format character added later is not removed until it is listed here.
const fn is_invisible(character: char) -> bool {
    matches!(
        character,
        // Cf: soft hyphen; Arabic number signs; Arabic letter mark; Arabic end of ayah;
        // Syriac abbreviation mark; Arabic pound and piastre marks above; Arabic disputed
        // end of ayah; Mongolian vowel separator.
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            // Cf: zero width space, non-joiner and joiner; left-to-right and right-to-left
            // marks; embeddings and overrides; word joiner and invisible operators; isolates
            // and the deprecated format characters; byte order mark (zero width no-break
            // space); interlinear annotation.
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            // Cf: Kaithi number signs; Egyptian hieroglyph format controls; shorthand format
            // controls; musical symbol format controls; the whole tag block (language tag and
            // tag characters).
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E007F}'
            // Private use: the BMP area and planes 15 and 16.
            | '\u{E000}'..='\u{F8FF}'
            | '\u{F0000}'..='\u{FFFFD}'
            | '\u{100000}'..='\u{10FFFD}'
            // Noncharacters.
            | '\u{FDD0}'..='\u{FDEF}'
    ) || (character as u32) & 0xFFFE == 0xFFFE
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

    /// Every code point of `ranges`, whatever text stands around it.
    fn each_of(ranges: &[(u32, u32)]) -> impl Iterator<Item = char> + '_ {
        ranges
            .iter()
            .flat_map(|&(first, last)| first..=last)
            .filter_map(char::from_u32)
    }

    #[test]
    fn mcp_008_f2_invisible_and_line_breaking_characters_never_survive() {
        let hidden = [
            (0x00AD, 0x00AD),
            (0x200B, 0x200F),
            (0x2060, 0x2064),
            (0x2066, 0x206F),
            (0xFEFF, 0xFEFF),
            (0xE0000, 0xE007F),
            (0xE000, 0xF8FF),
            (0xF0000, 0xFFFFF),
            (0x10_0000, 0x10_FFFF),
            (0xFDD0, 0xFDEF),
        ];
        let surroundings = ["", "a", "é", "日本", " ", "\u{1b}"];
        for character in each_of(&hidden) {
            for before in surroundings {
                let text = format!("{before}{character}x{character}");
                let cleaned = clean(&text);
                assert!(
                    !cleaned.contains(character),
                    "U+{:04X} survived in {cleaned:?}",
                    u32::from(character)
                );
                assert!(cleaned.ends_with('x'), "U+{:04X}", u32::from(character));
            }
        }
        for separator in ['\u{2028}', '\u{2029}', '\u{85}'] {
            assert_eq!(clean(&format!("a{separator}b")), "a b");
        }
        assert_eq!(
            clean("ok\u{E0049}\u{E0047}\u{E004E}\u{E004F}\u{E0052}\u{E0045}"),
            "ok"
        );
    }

    #[test]
    fn mcp_008_f2_ordinary_text_stays_as_it_is() {
        let visible = [
            (0x20, 0x7E),
            (0xA0, 0xAC),
            (0xAE, 0x24F),
            (0x370, 0x3FF),
            (0x400, 0x4FF),
            (0x5D0, 0x5EA),
            (0x620, 0x64A),
            (0x3040, 0x30FF),
            (0x4E00, 0x4FFF),
            (0xAC00, 0xAD00),
            (0x1F300, 0x1F64F),
        ];
        for character in each_of(&visible) {
            let text = format!("a{character}b");
            assert_eq!(clean(&text), text, "U+{:04X}", u32::from(character));
        }
    }

    #[test]
    fn cmd_026_a_hint_quotes_a_word_with_spaces() {
        assert_eq!(shell_word("ping"), "ping");
        assert_eq!(shell_word("HTTP check"), "'HTTP check'");
        assert_eq!(shell_word("it's"), "'it'\\''s'");
    }
}
