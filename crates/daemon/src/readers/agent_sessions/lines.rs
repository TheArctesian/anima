//! Reads the complete lines a session file gained since the last read (AIS-001): from an
//! offset to the last newline, so a line the program is still writing waits for the next read.
//! A line longer than [`MAX_LINE`] (a pasted image, a huge tool result) is skipped unread.

use std::io::{self, BufRead, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::Path;

use daemon_core::agent_usage::Tally;

use super::files::open_no_link;

/// The longest line read; the longest line of a real session is a few MiB.
const MAX_LINE: u64 = 16 * 1024 * 1024;

/// Feeds `tally` each complete line of the file at `path` after `offset`, and gives the offset
/// the next read starts at. Before each line `halt` may stop the read (an excluded session, or
/// the daemon stopping): the offset reached is given then.
///
/// # Errors
/// The I/O error of opening, seeking or reading the file.
pub(super) fn read_new(
    path: &Path,
    offset: u64,
    tally: &mut Tally,
    halt: &mut dyn FnMut(&Tally) -> bool,
) -> io::Result<u64> {
    let mut file = open_no_link(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut offset = offset;
    let mut line = Vec::new();
    while !halt(tally) {
        line.clear();
        let read = (&mut reader)
            .take(MAX_LINE + 1)
            .read_until(b'\n', &mut line)? as u64;
        match line.last() {
            _ if read == 0 => break,
            Some(b'\n') => {
                tally.feed(&line[..line.len() - 1]);
                offset += read;
            }
            _ if read > MAX_LINE => match skip_line(&mut reader)? {
                Some(rest) => offset += read + rest,
                // The long line has no end yet: it waits for the next read.
                None => break,
            },
            // A line the program is still writing.
            _ => break,
        }
    }
    Ok(offset)
}

/// Skips past the next newline: the bytes passed, the newline included, or `None` when the
/// file ends first.
fn skip_line(reader: &mut impl BufRead) -> io::Result<Option<u64>> {
    let mut passed: u64 = 0;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(None);
        }
        if let Some(at) = buffer.iter().position(|byte| *byte == b'\n') {
            reader.consume(at + 1);
            return Ok(Some(passed + at as u64 + 1));
        }
        let length = buffer.len();
        reader.consume(length);
        passed += length as u64;
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use daemon_core::agent_usage::Source;

    use super::*;

    const REPLY: &str = r#"{"type":"assistant","timestamp":"2026-11-08T09:00:00Z","message":{"model":"m","usage":{"output_tokens":1}}}"#;

    fn output(tally: &Tally) -> u64 {
        tally
            .days()
            .values()
            .map(|day| day.models["m"].output)
            .sum()
    }

    #[test]
    fn ais_001_a_file_is_read_from_where_the_last_read_stopped() {
        let mut file = tempfile::NamedTempFile::new().expect("a file");
        write!(file, "{REPLY}\n{REPLY}\n{}", &REPLY[..20]).expect("written");
        let mut tally = Tally::new(Source::ClaudeCode);
        let offset = read_new(file.path(), 0, &mut tally, &mut |_| false).expect("read");
        assert_eq!(output(&tally), 2, "the unfinished line waits");
        assert_eq!(offset, 2 * (REPLY.len() as u64 + 1));
        writeln!(file, "{}", &REPLY[20..]).expect("finished");
        let next = read_new(file.path(), offset, &mut tally, &mut |_| false).expect("read");
        assert_eq!(output(&tally), 3, "each line once");
        assert_eq!(next, 3 * (REPLY.len() as u64 + 1));
    }

    #[test]
    fn ais_001_a_halt_stops_the_read_at_a_line_end() {
        let mut file = tempfile::NamedTempFile::new().expect("a file");
        write!(file, "{REPLY}\n{REPLY}\n").expect("written");
        let mut tally = Tally::new(Source::ClaudeCode);
        let mut lines = 0;
        let mut halt = |_: &Tally| {
            lines += 1;
            lines > 1
        };
        let offset = read_new(file.path(), 0, &mut tally, &mut halt).expect("read");
        assert_eq!((output(&tally), offset), (1, REPLY.len() as u64 + 1));
    }

    #[test]
    fn ais_022_a_session_file_swapped_for_a_link_is_not_read() {
        let folder = tempfile::tempdir().expect("a folder");
        let target = folder.path().join("target.jsonl");
        std::fs::write(&target, format!("{REPLY}\n")).expect("written");
        let link = folder.path().join("s1.jsonl");
        std::os::unix::fs::symlink(&target, &link).expect("a link");
        let mut tally = Tally::new(Source::ClaudeCode);
        assert!(read_new(&link, 0, &mut tally, &mut |_| false).is_err());
        assert_eq!(tally.days().len(), 0);
    }

    #[test]
    fn ais_001_a_line_past_the_limit_is_skipped_and_the_next_is_read() {
        let mut file = tempfile::NamedTempFile::new().expect("a file");
        let long = "x".repeat(usize::try_from(MAX_LINE).expect("fits") + 10);
        write!(file, "{long}\n{REPLY}\n").expect("written");
        let mut tally = Tally::new(Source::ClaudeCode);
        let offset = read_new(file.path(), 0, &mut tally, &mut |_| false).expect("read");
        assert_eq!(output(&tally), 1);
        assert_eq!(offset, long.len() as u64 + 1 + REPLY.len() as u64 + 1);
    }
}
