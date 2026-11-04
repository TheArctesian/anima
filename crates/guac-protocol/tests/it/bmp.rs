//! `Instruction::with_bmp_text`: the client-bound text whose lengths agree in code points and
//! UTF-16 units, so guacamole-common-js before 1.5.2 parses it as guacd meant (R-046).

use std::borrow::Cow;

use guac_protocol::{Instruction, encode};
use proptest::prelude::*;

use crate::arbitrary::text;

#[test]
fn r_046_bmp_text_comes_back_borrowed_and_unchanged() {
    let name = Instruction::new("name", ["root@host: ~ é€ \u{FFFF}"]);
    let guarded = name.with_bmp_text();
    assert!(matches!(guarded, Cow::Borrowed(_)), "no copy for BMP text");
    assert_eq!(*guarded, name);
}

#[test]
fn r_046_each_non_bmp_character_becomes_one_replacement_character() {
    let name = Instruction::new("😀", ["😀😀", "\u{10FFFF}", "\u{10000}"]);
    let guarded = name.with_bmp_text();
    assert!(matches!(guarded, Cow::Owned(_)));
    assert_eq!(
        *guarded,
        Instruction::new("\u{FFFD}", ["\u{FFFD}\u{FFFD}", "\u{FFFD}", "\u{FFFD}"])
    );
}

#[test]
fn r_046_bmp_characters_keep_their_places_in_mixed_text() {
    let name = Instruction::new("name", ["a😀b", "plain", "é𝄞;x,1."]);
    assert_eq!(
        encode(&name.with_bmp_text()),
        "4.name,3.a\u{FFFD}b,5.plain,7.é\u{FFFD};x,1.;"
    );
}

/// The number of UTF-16 units in `value`, as JavaScript's `String.length` counts them.
fn utf16_units(value: &str) -> usize {
    value.encode_utf16().count()
}

proptest! {
    #[test]
    fn r_046_guarded_lengths_agree_in_code_points_and_utf16_units(
        opcode in text(16),
        args in prop::collection::vec(text(32), 0..6),
    ) {
        let instruction = Instruction::new(opcode, args);
        let guarded = instruction.with_bmp_text();
        prop_assert_eq!(guarded.args.len(), instruction.args.len());
        for element in std::iter::once(&guarded.opcode).chain(&guarded.args) {
            prop_assert_eq!(element.chars().count(), utf16_units(element));
        }
        let all_bmp = std::iter::once(&instruction.opcode)
            .chain(&instruction.args)
            .all(|element| element.chars().all(|c| c.len_utf16() == 1));
        prop_assert_eq!(matches!(guarded, Cow::Borrowed(_)), all_bmp);
    }
}
