//! One line at a time for a command that runs until interrupted (`event watch`, `device watch`): each line is
//! written and flushed at once, so a pipe or a terminal sees it when it happens.

use std::io::{self, Write as _};

use serde::Serialize;

use crate::failure::Failure;
use crate::output::text::clean;

/// Whether the reader is still there after a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reader {
    Open,
    /// The pipe closed (`anima event watch | head -1`): the command should stop.
    Gone,
}

/// Writes `line` and a line break to standard output. The line is cleaned here, whatever the
/// command made it from: a hub or a device chose some of its words (S3).
///
/// # Errors
/// [`Failure::Write`] when standard output fails, other than a closed pipe.
pub fn line(line: &str) -> Result<Reader, Failure> {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(clean(line).as_bytes())
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

/// Writes `value` as one JSON object on a line.
///
/// # Errors
/// [`Failure::Write`] when `value` cannot be written as JSON or standard output fails.
pub fn json_line<T: Serialize>(value: &T) -> Result<Reader, Failure> {
    let text = serde_json::to_string(value).map_err(|error| Failure::Write {
        reason: error.to_string(),
    })?;
    line(&text)
}

/// One change as one line: JSON of `value` with `--json`, otherwise `text(value)`.
///
/// # Errors
/// As [`json_line`].
pub fn emit<T: Serialize>(
    json: bool,
    value: &T,
    text: impl FnOnce(&T) -> String,
) -> Result<Reader, Failure> {
    if json {
        json_line(value)
    } else {
        line(&text(value))
    }
}
