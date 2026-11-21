//! The output of one thread that the daemon did not confirm yet (THR-113, H10). Pure: a list of
//! whole lines with sequence numbers and a limit of bytes, in memory only.

use std::collections::VecDeque;

use super::wire::{Line, Stream};

/// The limit of bytes that one thread keeps for the daemon (THR-113: 8 MiB).
pub const LIMIT: usize = 8 * 1024 * 1024;

/// The most bytes of one line. A longer line is replaced by [`TOO_LONG`], which the decoders of
/// the drive protocols do not know, so the turn stops (THR-046).
pub const MAX_LINE: usize = 4 * 1024 * 1024;

/// The line that takes the place of a line over [`MAX_LINE`].
pub const TOO_LONG: &str = r#"{"type":"anima_line_too_long"}"#;

/// The lines of one thread that wait for the confirmation of the daemon.
#[derive(Debug)]
pub struct Outbox {
    limit: usize,
    bytes: usize,
    next: u64,
    lines: VecDeque<(u64, Line)>,
}

impl Outbox {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            bytes: 0,
            next: 1,
            lines: VecDeque::new(),
        }
    }

    /// Whether the outbox holds its limit: the reader of the program then waits.
    #[must_use]
    pub fn full(&self) -> bool {
        self.bytes >= self.limit
    }

    /// Keeps one line and gives its sequence number. A line over [`MAX_LINE`] is kept as
    /// [`TOO_LONG`].
    pub fn push(&mut self, stream: Stream, text: String) -> u64 {
        let text = if text.len() > MAX_LINE {
            TOO_LONG.to_owned()
        } else {
            text
        };
        let seq = self.next;
        self.next += 1;
        self.bytes += text.len();
        self.lines.push_back((seq, Line { stream, text }));
        seq
    }

    /// Drops each line up to and with `seq`. Gives whether the outbox was full before.
    pub fn ack(&mut self, seq: u64) -> bool {
        let was_full = self.full();
        while self.lines.front().is_some_and(|(first, _)| *first <= seq) {
            if let Some((_, line)) = self.lines.pop_front() {
                self.bytes -= line.text.len();
            }
        }
        was_full
    }

    /// Each line the daemon did not confirm, with the sequence number of the first.
    #[must_use]
    pub fn pending(&self) -> Option<(u64, Vec<Line>)> {
        let first = self.lines.front()?.0;
        Some((
            first,
            self.lines.iter().map(|(_, line)| line.clone()).collect(),
        ))
    }

    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(text: &str) -> (Stream, String) {
        (Stream::Out, text.to_owned())
    }

    #[test]
    fn thr_113_lines_wait_until_the_daemon_confirms_them_and_come_again_in_order() {
        let mut outbox = Outbox::new(LIMIT);
        for text in ["a", "b", "c"] {
            let (stream, text) = out(text);
            outbox.push(stream, text);
        }
        outbox.ack(1);
        let (first, lines) = outbox.pending().expect("two lines wait");
        assert_eq!(first, 2);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
        outbox.ack(3);
        assert_eq!(outbox.pending(), None);
        assert_eq!(outbox.bytes(), 0);
    }

    #[test]
    fn thr_113_the_outbox_is_full_at_its_limit_and_free_again_after_a_confirmation() {
        let mut outbox = Outbox::new(10);
        let first = outbox.push(Stream::Out, "12345".to_owned());
        assert!(!outbox.full());
        outbox.push(Stream::Err, "67890".to_owned());
        assert!(outbox.full());
        assert!(outbox.ack(first), "it was full");
        assert!(!outbox.full());
    }

    #[test]
    fn thr_046_a_line_over_the_limit_becomes_a_line_that_stops_the_turn() {
        let mut outbox = Outbox::new(LIMIT);
        outbox.push(Stream::Out, "x".repeat(MAX_LINE + 1));
        let (_, lines) = outbox.pending().expect("one line");
        assert_eq!(lines[0].text, TOO_LONG);
    }
}
