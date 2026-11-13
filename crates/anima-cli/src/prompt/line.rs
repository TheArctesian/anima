//! The line a person types at a prompt in raw mode (CMD-010): the bytes the terminal sends
//! become keys, and the keys edit one answer. Pure: no terminal, no I/O.

use zeroize::Zeroizing;

/// What the person pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A character of the answer: printable, from one to four UTF-8 bytes.
    Char(char),
    /// Backspace or Delete: removes the last character.
    Erase,
    /// Ctrl-U: removes the whole answer.
    EraseLine,
    /// Return or Enter: the answer is complete.
    Enter,
    /// Ctrl-C: the question is cancelled.
    Interrupt,
    /// Ctrl-D: the end of input, when the answer is empty.
    End,
}

/// Where the decoder is inside an escape sequence (an arrow or function key).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Escape {
    #[default]
    Outside,
    /// After `ESC`.
    Started,
    /// After `ESC [` or `ESC O`, until the final byte.
    Sequence,
}

/// Turns the terminal's bytes into keys, one byte at a time, across questions: a character
/// split between reads, and the `\n` of a pasted `\r\n`, are handled where they fall.
#[derive(Debug, Default)]
pub struct Keys {
    pending: [u8; 4],
    filled: usize,
    expected: usize,
    escape: Escape,
    after_return: bool,
}

impl Keys {
    /// The key that `byte` completes, if any. Malformed UTF-8, escape sequences and other
    /// control bytes give no key.
    pub fn push(&mut self, byte: u8) -> Option<Key> {
        let after_return = std::mem::take(&mut self.after_return);
        if self.escape != Escape::Outside {
            return self.escaped(byte);
        }
        if self.expected > 0 {
            return self.continued(byte);
        }
        match byte {
            0x03 => Some(Key::Interrupt),
            0x04 => Some(Key::End),
            0x08 | 0x7f => Some(Key::Erase),
            0x15 => Some(Key::EraseLine),
            b'\r' => {
                self.after_return = true;
                Some(Key::Enter)
            }
            b'\n' => (!after_return).then_some(Key::Enter),
            0x1b => {
                self.escape = Escape::Started;
                None
            }
            0x00..=0x1f => None,
            0x20..=0x7e => Some(Key::Char(char::from(byte))),
            _ => self.started(byte),
        }
    }

    fn escaped(&mut self, byte: u8) -> Option<Key> {
        match (self.escape, byte) {
            (Escape::Started, b'[' | b'O') => {
                self.escape = Escape::Sequence;
                None
            }
            // `ESC` and one key (Alt with a key): both are dropped.
            (Escape::Started, 0x20..=0x7e) | (Escape::Sequence, 0x40..=0x7e) => {
                self.escape = Escape::Outside;
                None
            }
            (Escape::Sequence, 0x20..=0x3f) => None,
            // Not an escape sequence after all: the byte counts as itself.
            _ => {
                self.escape = Escape::Outside;
                self.push(byte)
            }
        }
    }

    /// The first byte of a multi-byte character.
    fn started(&mut self, byte: u8) -> Option<Key> {
        self.expected = match byte {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return None,
        };
        self.pending[0] = byte;
        self.filled = 1;
        None
    }

    fn continued(&mut self, byte: u8) -> Option<Key> {
        if !(0x80..=0xbf).contains(&byte) {
            // The character was cut short: it is dropped and `byte` starts afresh.
            self.pending = [0; 4];
            self.expected = 0;
            return self.push(byte);
        }
        self.pending[self.filled] = byte;
        self.filled += 1;
        if self.filled < self.expected {
            return None;
        }
        let complete = &self.pending[..self.expected];
        let key = std::str::from_utf8(complete)
            .ok()
            .and_then(|text| text.chars().next())
            .filter(|character| !character.is_control())
            .map(Key::Char);
        self.pending = [0; 4];
        self.expected = 0;
        key
    }
}

impl Drop for Keys {
    fn drop(&mut self) {
        self.pending = [0; 4];
    }
}

/// How a line ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// Return: the answer is the text.
    Answered,
    /// Return after more than the limit was typed: refused, never cut short.
    TooLong,
    /// Ctrl-C.
    Interrupted,
    /// Ctrl-D on an empty line, or the terminal closed.
    Ended,
}

/// One answer being typed, held in memory that is wiped when it drops.
pub struct Line {
    text: Zeroizing<String>,
    limit: usize,
    too_long: bool,
}

impl Line {
    /// An empty answer of at most `limit` bytes. The space is reserved now, so the text is
    /// never copied to a larger buffer that would not be wiped.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            text: Zeroizing::new(String::with_capacity(limit)),
            limit,
            too_long: false,
        }
    }

    /// Applies `key`; the ending when the line is over.
    pub fn apply(&mut self, key: Key) -> Option<Ending> {
        match key {
            Key::Char(character) => {
                if self.text.len() + character.len_utf8() > self.limit {
                    self.too_long = true;
                } else {
                    self.text.push(character);
                }
                None
            }
            Key::Erase => {
                self.text.pop();
                None
            }
            Key::EraseLine => {
                self.text.clear();
                self.too_long = false;
                None
            }
            Key::Enter if self.too_long => Some(Ending::TooLong),
            Key::Enter => Some(Ending::Answered),
            Key::Interrupt => Some(Ending::Interrupted),
            Key::End => (self.text.is_empty() && !self.too_long).then_some(Ending::Ended),
        }
    }

    /// The answer so far.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The answer, still wiped when it drops.
    #[must_use]
    pub fn into_text(self) -> Zeroizing<String> {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys `bytes` give.
    fn keys(bytes: &[u8]) -> Vec<Key> {
        let mut keys = Keys::default();
        bytes.iter().filter_map(|&byte| keys.push(byte)).collect()
    }

    /// The answer and how it ended, after `bytes` at a line of `limit` bytes.
    fn typed(bytes: &[u8], limit: usize) -> (String, Option<Ending>) {
        let mut line = Line::new(limit);
        let mut decoder = Keys::default();
        for &byte in bytes {
            if let Some(ending) = decoder.push(byte).and_then(|key| line.apply(key)) {
                return (line.text().to_owned(), Some(ending));
            }
        }
        (line.text().to_owned(), None)
    }

    #[test]
    fn cmd_010_return_ends_the_line_and_is_not_part_of_it() {
        assert_eq!(
            typed(b"secret\r", 64),
            ("secret".to_owned(), Some(Ending::Answered))
        );
        assert_eq!(
            typed(b"secret\n", 64),
            ("secret".to_owned(), Some(Ending::Answered))
        );
    }

    #[test]
    fn cmd_010_the_newline_of_a_pasted_return_newline_is_not_a_second_line() {
        assert_eq!(
            keys(b"a\r\nb\r"),
            [Key::Char('a'), Key::Enter, Key::Char('b'), Key::Enter]
        );
        assert_eq!(keys(b"\r\r"), [Key::Enter, Key::Enter]);
        assert_eq!(keys(b"\rx\n"), [Key::Enter, Key::Char('x'), Key::Enter]);
    }

    #[test]
    fn cmd_010_backspace_delete_and_ctrl_u_edit_the_answer() {
        assert_eq!(typed(b"secrex\x7f\x08t\r", 64).0, "secrt");
        assert_eq!(typed(b"wrong\x15right\r", 64).0, "right");
        assert_eq!(typed(b"\x7f\x7fok\r", 64).0, "ok");
        assert_eq!(typed("pä\x7f\r".as_bytes(), 64).0, "p");
    }

    #[test]
    fn cmd_010_ctrl_c_interrupts_and_ctrl_d_ends_only_an_empty_line() {
        assert_eq!(typed(b"abc\x03", 64).1, Some(Ending::Interrupted));
        assert_eq!(typed(b"\x04", 64).1, Some(Ending::Ended));
        assert_eq!(
            typed(b"ab\x04c\r", 64),
            ("abc".to_owned(), Some(Ending::Answered))
        );
    }

    #[test]
    fn cmd_010_multi_byte_characters_typed_or_pasted_are_kept_whole() {
        let password = "pässwörd 鍵 🔑";
        let mut bytes = password.as_bytes().to_vec();
        bytes.push(b'\r');
        assert_eq!(
            typed(&bytes, 64),
            (password.to_owned(), Some(Ending::Answered))
        );
    }

    #[test]
    fn cmd_010_malformed_utf8_control_bytes_and_escape_sequences_are_not_part_of_the_answer() {
        // An arrow key, a function key, Alt-x, a cut-short character, a stray continuation
        // byte, an invalid lead byte, a C1 control, a tab and a bell.
        let bytes = b"a\x1b[A\x1b[15~\x1bOP\x1bxb\xc3c\x80\xffd\xc2\x85\t\x07e\r";
        assert_eq!(
            typed(bytes, 64),
            ("abcde".to_owned(), Some(Ending::Answered))
        );
    }

    #[test]
    fn cmd_010_an_answer_over_the_limit_is_refused_not_cut_short() {
        assert_eq!(
            typed(b"abcd\r", 4),
            ("abcd".to_owned(), Some(Ending::Answered))
        );
        assert_eq!(typed(b"abcde\r", 4).1, Some(Ending::TooLong));
        // A character is never split to fit.
        assert_eq!(typed("abcä\r".as_bytes(), 4).1, Some(Ending::TooLong));
        // Erasing does not hide that more was typed; clearing the line starts afresh.
        assert_eq!(typed(b"abcde\x7f\x7f\r", 4).1, Some(Ending::TooLong));
        assert_eq!(
            typed(b"abcde\x15ok\r", 4),
            ("ok".to_owned(), Some(Ending::Answered))
        );
    }

    /// A small deterministic generator, so the fuzz run is the same every time.
    fn next(state: &mut u64) -> u8 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 56).to_le_bytes()[0]
    }

    #[test]
    fn cmd_010_any_bytes_give_a_short_printable_answer_and_never_panic() {
        let mut state = 0x5eed;
        for round in 0..2_000 {
            let length = usize::from(next(&mut state)) % 96;
            let bytes: Vec<u8> = (0..length).map(|_| next(&mut state)).collect();
            let limit = 1 + round % 40;
            let (text, _) = typed(&bytes, limit);
            assert!(text.len() <= limit, "{bytes:?}");
            assert!(!text.chars().any(char::is_control), "{bytes:?}");
        }
    }
}
