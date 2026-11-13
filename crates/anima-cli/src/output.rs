//! Where a command's results go (CMD-020, CMD-022): data on standard output as a table or, with
//! `--json`, as JSON with the K1 field names; messages and errors on standard error.

pub mod stream;
pub mod table;
pub mod text;

use std::fmt::Write as _;
use std::io::{self, Write as _};

use serde::Serialize;

use crate::failure::{Failure, Hint};
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
    message: &'a str,
    hint: Option<&'a str>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    fields: &'a [FieldError],
}

/// A failure as it is shown: every string that may come from the hub is clean.
struct Shown {
    code: &'static str,
    message: String,
    hint: Option<Hint>,
    fields: Vec<FieldError>,
}

impl Shown {
    fn of(failure: &Failure) -> Self {
        let hint = failure.hint().map(|hint| match hint {
            Hint::Run(command) => Hint::Run(text::clean(&command)),
            Hint::Say(sentence) => Hint::Say(text::clean(&sentence)),
        });
        Self {
            code: failure.code(),
            message: text::clean(&failure.to_string()),
            hint,
            fields: failure
                .fields()
                .iter()
                .map(|field| FieldError {
                    field: text::clean(&field.field),
                    message: text::clean(&field.message),
                })
                .collect(),
        }
    }

    /// The CMD-022 JSON object on one line.
    fn json(&self) -> String {
        let body = ErrorBody {
            error: ErrorFields {
                code: self.code,
                message: &self.message,
                hint: self.hint.as_ref().map(Hint::text),
                fields: &self.fields,
            },
        };
        // Strings and a list of string pairs: this cannot fail, and there is nowhere else to
        // report it.
        serde_json::to_string(&body).unwrap_or_default()
    }

    /// The sentence, the rejected fields and the hint, one to a line.
    fn lines(&self) -> String {
        let mut lines = format!("error: {}\n", self.message);
        for field in &self.fields {
            let _ = writeln!(lines, "  {}: {}", field.field, field.message);
        }
        match &self.hint {
            Some(Hint::Run(command)) => {
                let _ = writeln!(lines, "hint: run `{command}`");
            }
            Some(Hint::Say(sentence)) => {
                let _ = writeln!(lines, "hint: {sentence}");
            }
            None => {}
        }
        lines
    }
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

    /// Prints `text` and a line break, as it is.
    ///
    /// # Errors
    /// As [`Output::data`].
    pub fn line(self, text: &str) -> Result<(), Failure> {
        write_out(&format!("{text}\n"))
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
            write_err(&format!("{}\n", text::clean_lines(message)));
        }
    }

    /// The failure on standard error: a sentence and a hint, or the CMD-022 JSON object. Every
    /// string is cleaned here, so no command has to (S2).
    pub fn failure(self, failure: &Failure) {
        let shown = Shown::of(failure);
        if self.json {
            write_err(&format!("{}\n", shown.json()));
        } else {
            write_err(&shown.lines());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(message: &str, hint: Hint) -> Failure {
        Failure::Usage {
            message: message.to_owned(),
            hint: Some(hint),
        }
    }

    #[test]
    fn cmd_022_s2_a_hint_with_escape_bytes_is_cleaned_in_text_and_json() {
        let hostile = usage(
            "Two devices.\u{1b}[2J",
            Hint::run("anima device get a\u{1b}]0;pwned\u{7}b\u{202E}"),
        );
        let shown = Shown::of(&hostile);
        for text in [shown.lines(), shown.json()] {
            assert!(!text.contains(['\u{1b}', '\u{7}', '\u{202E}']), "{text:?}");
        }
        assert!(
            shown
                .lines()
                .contains("hint: run `anima device get a ]0;pwned b`")
        );
    }

    #[test]
    fn cmd_022_c6_a_sentence_hint_is_not_printed_as_a_command() {
        let shown = Shown::of(&usage(
            "Not that.",
            Hint::say("Pass --yes to skip the question."),
        ));
        assert!(
            shown
                .lines()
                .ends_with("hint: Pass --yes to skip the question.\n")
        );
        assert!(!shown.lines().contains("run `"));
        assert!(
            shown
                .json()
                .contains(r#""hint":"Pass --yes to skip the question.""#)
        );
    }

    #[test]
    fn cmd_021_c5_a_declined_confirmation_is_exit_1_with_the_declined_code() {
        assert_eq!(Failure::Declined.exit_code(), 1);
        let shown = Shown::of(&Failure::Declined);
        assert!(
            shown.json().contains(r#""code":"DECLINED""#),
            "{}",
            shown.json()
        );
    }
}
