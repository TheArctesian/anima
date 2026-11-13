//! Where a command's results go (CMD-020, CMD-022): data on standard output as a table or, with
//! `--json`, as JSON with the K1 field names; messages and errors on standard error.

pub mod table;
pub mod text;

use std::fmt::Write as _;
use std::io::{self, Write as _};

use serde::Serialize;

use crate::failure::Failure;
use crate::hub::FieldError;
pub use table::{Cell, Table, Tone};

/// How this run prints.
#[derive(Clone, Copy, Debug)]
pub struct Output {
    json: bool,
    colour: bool,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorFields<'a>,
}

#[derive(Serialize)]
struct ErrorFields<'a> {
    code: &'a str,
    message: String,
    hint: Option<String>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    fields: &'a [FieldError],
}

impl Output {
    /// `colour` only for a terminal and when `NO_COLOR` is not set (CMD-027).
    #[must_use]
    pub const fn new(json: bool, stdout_terminal: bool, no_colour: bool) -> Self {
        Self {
            json,
            colour: stdout_terminal && !no_colour,
        }
    }

    #[must_use]
    pub const fn json(self) -> bool {
        self.json
    }

    /// Prints `value` as JSON, or the table `table` makes from it.
    ///
    /// # Errors
    /// [`Failure::Write`] when standard output fails, other than a closed pipe.
    pub fn data<T: Serialize>(
        self,
        value: &T,
        table: impl FnOnce(&T) -> Table,
    ) -> Result<(), Failure> {
        if self.json {
            let text = serde_json::to_string_pretty(value).map_err(|error| Failure::Write {
                reason: error.to_string(),
            })?;
            write_out(&format!("{text}\n"))
        } else {
            write_out(&table(value).render(self.colour))
        }
    }

    /// Prints JSON as it is, for `anima api`.
    ///
    /// # Errors
    /// As [`Output::data`].
    pub fn raw(self, value: &serde_json::Value) -> Result<(), Failure> {
        // A `Value` has string keys only, so it always serialises.
        write_out(&format!("{value:#}\n"))
    }

    /// A message for a person on standard error; none with `--json`.
    pub fn note(self, message: &str) {
        if !self.json {
            write_err(&format!("{}\n", text::clean(message)));
        }
    }

    /// The failure on standard error: a sentence and a hint, or the CMD-022 JSON object.
    pub fn failure(self, failure: &Failure) {
        let message = text::clean(&failure.to_string());
        let hint = failure.hint();
        if self.json {
            let body = ErrorBody {
                error: ErrorFields {
                    code: failure.code(),
                    message,
                    hint,
                    fields: failure.fields(),
                },
            };
            // Strings and a list of string pairs: this cannot fail, and there is nowhere
            // else to report it.
            let text = serde_json::to_string(&body).unwrap_or_default();
            write_err(&format!("{text}\n"));
            return;
        }
        let mut lines = format!("error: {message}\n");
        for field in failure.fields() {
            let field = format!("  {}: {}", field.field, field.message);
            let _ = writeln!(lines, "{}", text::clean(&field));
        }
        if let Some(hint) = hint {
            let _ = writeln!(lines, "hint: run `{hint}`");
        }
        write_err(&lines);
    }
}

/// Writes to standard output. A closed pipe (`anima device list | head`) is not an error.
fn write_out(text: &str) -> Result<(), Failure> {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(Failure::Write {
            reason: error.to_string(),
        }),
        _ => Ok(()),
    }
}

/// Writes to standard error. A failure here has no other place to be reported.
fn write_err(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}
