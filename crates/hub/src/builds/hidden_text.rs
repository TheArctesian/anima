//! Text in builder input that a reader never sees: terminal escape sequences (ECMA-48),
//! Unicode format characters (category Cf) and the other default-ignorable characters.
//! Removing them first lets redaction read a line as a terminal shows it, so
//! `\x1b[32mGITHUB_TOKEN\x1b[0m=…` is the key `GITHUB_TOKEN` (BLD-049). Redaction also reads
//! the line with them in place (`log_lines::clean`), in case this reading hides too much.
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

/// The default-ignorable characters outside category Cf that show nothing: the combining
/// grapheme joiner, the Hangul fillers and the variation selectors.
const IGNORABLE: [(char, char); 6] = [
    ('\u{34f}', '\u{34f}'),
    ('\u{115f}', '\u{1160}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{fe00}', '\u{fe0f}'),
    ('\u{ffa0}', '\u{ffa0}'),
    ('\u{e0100}', '\u{e01ef}'),
];

/// The bytes that end an escape `ESC <byte>` as terminals read it: `ESC 7` and `ESC 8` (save
/// and restore the cursor), the other digits, `=` and `>` (keypad modes), `c` (reset), `D`,
/// `E`, `H` and `M` (line feed, next line, tab stop, reverse line feed). Any other byte after
/// `ESC` is shown, so an `ESC` cannot hide the first letter of a key such as `password`.
fn ends_escape(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '=' | '>' | 'c' | 'D' | 'E' | 'H' | 'M')
}

/// Whether `c` shows nothing yet changes how text reads: a Unicode format character (category
/// Cf; U+202E turns the text after it around) or another default-ignorable character
/// (U+FE0F, U+3164).
pub(super) fn is_hidden(c: char) -> bool {
    FORMAT
        .iter()
        .chain(&IGNORABLE)
        .any(|&(first, last)| (first..=last).contains(&c))
}

/// What a reading does with a byte that ends an escape and could be text: the final byte of a
/// control sequence (`ESC [ p`), a designator (`ESC ( B`) or the byte after a bare `ESC`
/// (`ESC 7`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Finals {
    /// Removed with the sequence, as a terminal does.
    Drop,
    /// Kept as text.
    Keep,
    /// Removed when real output ends sequences with it (`m` of a colour, `B` of `ESC ( B`,
    /// the digit of `ESC 7`), else kept as text: an unusual final is more likely a letter the
    /// sequence took.
    KeepRare,
}

impl Finals {
    /// Whether the reading keeps `c` as text, `usual` saying whether real output ends this
    /// kind of sequence with it.
    fn keeps(self, c: char, usual: impl Fn(char) -> bool) -> bool {
        match self {
            Self::Drop => false,
            Self::Keep => true,
            Self::KeepRare => !usual(c),
        }
    }
}

/// `text` as a terminal shows it: without its escape sequences and hidden characters. A
/// sequence that is never ended takes the rest of the text with it: the terminal would not
/// show that either.
pub(super) fn visible(text: &str) -> String {
    read(text, Finals::Drop, false)
}

/// `text` with each escape sequence and hidden character turned into one space, the bytes that
/// may end a sequence kept. Where [`visible`] takes a key's first letter as a final byte
/// (`ESC [` then `password`), this reading keeps the key whole and apart from what precedes it.
pub(super) fn spaced(text: &str) -> String {
    read(text, Finals::Keep, true)
}

/// `text` without its escape introducers, parameters and hidden characters, the bytes that may
/// end a sequence kept: `ESC [ pass ZWSP word` reads `password`.
pub(super) fn joined(text: &str) -> String {
    read(text, Finals::Keep, false)
}

/// As [`joined`], but the final bytes that real output uses are removed as a terminal does:
/// `ESC [ pass ESC [0m word` reads `password`.
pub(super) fn joined_rare(text: &str) -> String {
    read(text, Finals::KeepRare, false)
}

fn read(text: &str, finals: Finals, spaced: bool) -> String {
    let gap = |shown: &mut String| {
        if spaced {
            shown.push(' ');
        }
    };
    let mut shown = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ESC => {
                skip_escape(&mut chars, finals, &mut shown);
                gap(&mut shown);
            }
            // C1 CSI, and the C1 starts of OSC, DCS, SOS, PM and APC strings.
            '\u{9b}' => {
                skip_control_sequence(&mut chars, finals);
                gap(&mut shown);
            }
            intro @ ('\u{9d}' | '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}') => {
                skip_string(
                    &mut chars,
                    finals.keeps(intro, |c| c == '\u{9d}'),
                    &mut shown,
                );
                gap(&mut shown);
            }
            c if is_hidden(c) => gap(&mut shown),
            c => shown.push(c),
        }
    }
    shown
}

/// After `ESC`: a control sequence (`ESC [`), a string (`ESC ]`, `ESC P`, `ESC X`, `ESC ^`,
/// `ESC _`), a character set designation with its one designator byte (`ESC ( B`), or one of
/// the final bytes of [`ends_escape`] (`ESC 7`). Otherwise only the `ESC` goes. A designator or
/// final byte goes only as `finals` says, and so does the text of a string, into `kept`.
fn skip_escape(chars: &mut Peekable<Chars<'_>>, finals: Finals, kept: &mut String) {
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            skip_control_sequence(chars, finals);
        }
        // A reading that keeps the byte after `ESC` reads the string's introducer and text as
        // text: `P` and `X` may be a key's letters (`PG ESC PASSWORD`).
        Some(intro @ (']' | 'P' | 'X' | '^' | '_')) if !finals.keeps(intro, |c| c == ']') => {
            chars.next();
            skip_string(chars, false, kept);
        }
        Some('(' | ')' | '*' | '+') => {
            chars.next();
            chars.next_if(|&c| {
                ('\u{30}'..='\u{7e}').contains(&c) && !finals.keeps(c, |c| matches!(c, 'B' | '0'))
            });
        }
        _ => {
            chars.next_if(|&c| {
                ends_escape(c) && !finals.keeps(c, |c| c.is_ascii_digit() || matches!(c, '=' | '>'))
            });
        }
    }
}

/// The parameter and intermediate bytes of a control sequence, then its final byte
/// (`[31m`, `[2K`, `[?25l`) unless `finals` keeps it.
fn skip_control_sequence(chars: &mut Peekable<Chars<'_>>, finals: Finals) {
    while chars
        .next_if(|c| ('\u{20}'..='\u{3f}').contains(c))
        .is_some()
    {}
    chars.next_if(|&c| ('\u{40}'..='\u{7e}').contains(&c) && !finals.keeps(c, |c| c == 'm'));
}

/// A string up to its terminator: `BEL`, `ESC \` or the C1 string terminator. Any other `ESC`
/// ends it too and is left in place, as terminals read it as the start of the next sequence
/// (`ESC ] ESC [ 31m` is a cut-off string, then a colour). With `keep_text` the string's text
/// goes into `kept`: a string can hide part of a key (`api ESC _ k ESC \ ey`). Real output
/// sends OSC strings (titles, links); the other kinds are rare.
fn skip_string(chars: &mut Peekable<Chars<'_>>, keep_text: bool, kept: &mut String) {
    while let Some(&c) = chars.peek() {
        match c {
            '\u{7}' | '\u{9c}' => {
                chars.next();
                return;
            }
            ESC => {
                let mut ahead = chars.clone();
                ahead.next();
                if ahead.peek() == Some(&'\\') {
                    chars.nth(1);
                }
                return;
            }
            c => {
                chars.next();
                if keep_text {
                    kept.push(c);
                }
            }
        }
    }
}
