//! Questions at the terminal (CMD-023). A question is asked only when standard input is a
//! terminal and `--no-input` is not set; otherwise the command fails and names the flag that
//! gives the answer.

use std::io::{self, BufRead as _, Read as _, Write as _};

use zeroize::Zeroizing;

use crate::failure::Failure;

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
        eprint!("{what}: ");
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
        rpassword::prompt_password(format!("{what}: "))
            .map(Zeroizing::new)
            .map_err(|error| terminal_failure(&error))
    }

    /// Whether the operator agrees to `question`: `--yes`, or `y` at the terminal (CMD-024).
    ///
    /// # Errors
    /// [`Failure::Usage`] naming `--yes` when it cannot ask.
    pub fn confirm(self, question: &str) -> Result<bool, Failure> {
        if self.yes {
            return Ok(true);
        }
        if !self.interactive {
            return Err(Failure::missing("A confirmation", "--yes"));
        }
        let answer = self.ask(&format!("{question} [y/N]"), "--yes")?;
        Ok(matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes"))
    }
}

fn terminal_failure(error: &io::Error) -> Failure {
    Failure::Input {
        what: "the terminal".to_owned(),
        reason: error.to_string(),
    }
}
