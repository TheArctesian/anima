//! Text in builder input that a reader never sees: terminal escape sequences (ECMA-48) and
//! Unicode format characters (category Cf). Removing them first lets redaction read a line as
//! a terminal shows it, so `\x1b[32mGITHUB_TOKEN\x1b[0m=…` is the key `GITHUB_TOKEN` (BLD-049).
//!
//! Pure.

use std::{iter::Peekable, str::Chars};

const ESC: char = '\u{1b}';

/// The format characters of Unicode 15 (general category Cf): soft hyphen, bidirectional
/// overrides and isolates, zero-width characters, tags and the like.
const FORMAT: [(char, char); 21] = [
    ('\u{ad}', '\u{ad}'),
    ('\u{600}', '\u{605}'),
    ('\u{61c}', '\u{61c}'),
    ('\u{6dd}', '\u{6dd}'),
    ('\u{70f}', '\u{70f}'),
    ('\u{890}', '\u{891}'),
    ('\u{8e2}', '\u{8e2}'),
    ('\u{180e}', '\u{180e}'),
    ('\u{200b}', '\u{200f}'),
    ('\u{202a}', '\u{202e}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206f}'),
    ('\u{feff}', '\u{feff}'),
    ('\u{fff9}', '\u{fffb}'),
    ('\u{110bd}', '\u{110bd}'),
    ('\u{110cd}', '\u{110cd}'),
    ('\u{13430}', '\u{1343f}'),
    ('\u{1bca0}', '\u{1bca3}'),
    ('\u{1d173}', '\u{1d17a}'),
    ('\u{e0001}', '\u{e0001}'),
    ('\u{e0020}', '\u{e007f}'),
];

/// Whether `c` is a Unicode format character (category Cf), which changes how text shows
/// without showing itself (U+202E turns the text after it around).
pub(super) fn is_format(c: char) -> bool {
    FORMAT
        .iter()
        .any(|&(first, last)| (first..=last).contains(&c))
}

/// `text` without its escape sequences and format characters. A sequence that is never ended
/// takes the rest of the text with it: the terminal would not show that either.
pub(super) fn visible(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ESC => skip_escape(&mut chars),
            // C1 CSI, and the C1 starts of OSC, DCS, SOS, PM and APC strings.
            '\u{9b}' => skip_control_sequence(&mut chars),
            '\u{9d}' | '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}' => skip_string(&mut chars),
            c if is_format(c) => {}
            c => shown.push(c),
        }
    }
    shown
}

/// After `ESC`: a control sequence (`ESC [`), a string (`ESC ]`, `ESC P`, `ESC X`, `ESC ^`,
/// `ESC _`), or intermediate bytes and one final byte (`ESC ( B`, `ESC 7`).
fn skip_escape(chars: &mut Peekable<Chars<'_>>) {
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            skip_control_sequence(chars);
        }
        Some(']' | 'P' | 'X' | '^' | '_') => {
            chars.next();
            skip_string(chars);
        }
        _ => {
            while chars
                .next_if(|c| ('\u{20}'..='\u{2f}').contains(c))
                .is_some()
            {}
            chars.next_if(|c| ('\u{30}'..='\u{7e}').contains(c));
        }
    }
}

/// The parameter and intermediate bytes of a control sequence, then its final byte
/// (`[31m`, `[2K`, `[?25l`).
fn skip_control_sequence(chars: &mut Peekable<Chars<'_>>) {
    while chars
        .next_if(|c| ('\u{20}'..='\u{3f}').contains(c))
        .is_some()
    {}
    chars.next_if(|c| ('\u{40}'..='\u{7e}').contains(c));
}

/// A string up to its terminator: `BEL`, `ESC \` or the C1 string terminator. An `ESC` ends
/// it in any case, as in terminals; the byte after it goes too when it is `\`.
fn skip_string(chars: &mut Peekable<Chars<'_>>) {
    while let Some(c) = chars.next() {
        match c {
            '\u{7}' | '\u{9c}' => return,
            ESC => {
                chars.next_if_eq(&'\\');
                return;
            }
            _ => {}
        }
    }
}
