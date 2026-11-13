//! One line at a time for a command that runs until interrupted (`event watch`): each line is
//! written and flushed at once, so a pipe or a terminal sees it when it happens.

use std::io::{self, Write as _};

use crate::failure::Failure;

/// Whether the reader is still there after a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reader {
    Open,
    /// The pipe closed (`anima event watch | head -1`): the command should stop.
    Gone,
}

/// Writes `line` and a line break to standard output.
///
/// # Errors
/// [`Failure::Write`] when standard output fails, other than a closed pipe.
pub fn line(line: &str) -> Result<Reader, Failure> {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(line.as_bytes())
        .and_then(|()| stdout.write_all(b"\n"))
        .and_then(|()| stdout.flush())
    {
        Ok(()) => Ok(Reader::Open),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(Reader::Gone),
        Err(error) => Err(Failure::Write {
            reason: error.to_string(),
        }),
    }
}
