//! Questions at the terminal (CMD-023). A question is asked only when standard input is a
//! terminal and `--no-input` is not set; otherwise the command fails and names the flag that
//! gives the answer.

use std::io::{self, BufRead as _, Read as _, Write as _};

use zeroize::Zeroizing;

use crate::failure::Failure;
use crate::output::text::clean;

/// The longest answer read from the terminal.
const MAX_ANSWER: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub struct Prompter {
    interactive: bool,
    yes: bool,
}

impl Prompter {
    #[must_use]
    pub const fn new(stdin_terminal: bool, no_input: bool, yes: bool) -> Self {
        Self {
            interactive: stdin_terminal && !no_input,
            yes,
        }
    }

    /// Whether a question can be asked.
    #[must_use]
    pub const fn interactive(self) -> bool {
        self.interactive
    }

    /// Asks for `what` with echo; fails naming `flag` when it cannot ask.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask; [`Failure::Input`] when the terminal fails.
    pub fn ask(self, what: &str, flag: &str) -> Result<String, Failure> {
        if !self.interactive {
            return Err(Failure::missing(what, flag));
        }
        eprint!("{}", prompt_text(what));
        io::stderr()
            .flush()
            .map_err(|error| terminal_failure(&error))?;
        let mut line = String::new();
        io::stdin()
            .lock()
            .take(MAX_ANSWER as u64)
            .read_line(&mut line)
            .map_err(|error| terminal_failure(&error))?;
        Ok(line.trim().to_owned())
    }

    /// Asks for a secret with echo off (CMD-010, CMD-025); fails naming `flag` when it cannot.
    ///
    /// # Errors
    /// As [`Prompter::ask`].
    pub fn ask_secret(self, what: &str, flag: &str) -> Result<Zeroizing<String>, Failure> {
        if !self.interactive {
            return Err(Failure::missing(what, flag));
        }
        rpassword::prompt_password(prompt_text(what))
            .map(Zeroizing::new)
            .map_err(|error| terminal_failure(&error))
    }

    /// Lets the command go on when the operator agrees to `question`: `--yes`, or `y` at the
    /// terminal (CMD-024). The one way every confirmation ends.
    ///
    /// # Errors
    /// [`Failure::Usage`] naming `--yes` when it cannot ask; [`Failure::Declined`] when the
    /// answer is not yes.
    pub fn confirm(self, question: &str) -> Result<(), Failure> {
        if self.yes {
            return Ok(());
        }
        if !self.interactive {
            return Err(Failure::missing("A confirmation", "--yes"));
        }
        let answer = self.ask(&format!("{question} [y/N]"), "--yes")?;
        decision(&answer)
    }
}

/// What a prompt shows for `what`. Callers build `what` from names the hub gave, so it is
/// cleaned here and no caller can forget (S1).
fn prompt_text(what: &str) -> String {
    format!("{}: ", clean(what))
}

/// Yes is `y` or `yes` in any case, nothing else; anything else declines (CMD-024).
fn decision(answer: &str) -> Result<(), Failure> {
    if matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        Err(Failure::Declined)
    }
}

fn terminal_failure(error: &io::Error) -> Failure {
    Failure::Input {
        what: "the terminal".to_owned(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_024_only_y_or_yes_in_any_case_confirms() {
        for yes in ["y", "Y", "yes", "YES", "Yes"] {
            assert!(decision(yes).is_ok(), "{yes}");
        }
        for no in ["", "n", "no", "yep", "ok", "yes please", "1"] {
            assert!(matches!(decision(no), Err(Failure::Declined)), "{no}");
        }
    }

    #[test]
    fn cmd_024_s1_a_question_with_escape_bytes_is_shown_clean() {
        let shown = prompt_text("Delete maintenance window \u{1b}[2Jgood\u{202E}?");
        assert_eq!(shown, "Delete maintenance window  [2Jgood?: ");
    }

    #[test]
    fn cmd_024_yes_confirms_and_no_terminal_without_yes_is_a_usage_failure() {
        assert!(Prompter::new(false, false, true).confirm("Delete?").is_ok());
        let refused = Prompter::new(false, false, false).confirm("Delete?");
        assert!(matches!(refused, Err(Failure::Usage { .. })));
    }
}
