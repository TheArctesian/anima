//! The text of a new issue or comment: from a file or standard input (`-`), never an argument,
//! so a long Markdown text keeps its shape and stays out of the shell's history (CMD-025). A
//! failure names the file, never the text (LOG-007).

use std::path::Path;

use crate::failure::Failure;
use crate::input;
use crate::prompt::Prompter;

/// GitHub takes at most 65,536 characters; a character is at most four bytes of UTF-8.
const MAX_BODY_BYTES: usize = 4 * 65_536;

/// The UTF-8 text in `from` (`-` for standard input); `what` names it in a failure.
///
/// # Errors
/// [`Failure::Input`] when it cannot be read, is too large or is not UTF-8;
/// [`Failure::Usage`] when it holds only white space.
pub fn read(prompter: Prompter, from: &Path, what: &str) -> Result<String, Failure> {
    let bytes = input::read(prompter, from, what, MAX_BODY_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Failure::Input {
        what: what.to_owned(),
        reason: "it is not UTF-8 text".to_owned(),
    })?;
    if text.trim().is_empty() {
        return Err(Failure::Usage {
            message: format!("{what} is empty: nothing was sent."),
            hint: None,
        });
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_025_a_body_that_is_not_text_or_is_blank_is_refused_without_quoting_it() {
        let dir = tempfile::tempdir().expect("a folder");
        let prompter = Prompter::new(false, false, false);
        let binary = dir.path().join("body.bin");
        std::fs::write(&binary, [0x66, 0x6f, 0xff, 0xfe]).expect("a file");
        let failure = read(prompter, &binary, "the issue's body").expect_err("not UTF-8");
        assert!(failure.to_string().contains("not UTF-8"), "{failure}");
        let blank = dir.path().join("blank.md");
        std::fs::write(&blank, " \n\t\n").expect("a file");
        let failure = read(prompter, &blank, "the issue's body").expect_err("blank");
        assert_eq!(failure.exit_code(), 2);
        let text = dir.path().join("body.md");
        std::fs::write(&text, "# Title\n\nSteps\n").expect("a file");
        let read = read(prompter, &text, "the issue's body").expect("text");
        assert_eq!(read, "# Title\n\nSteps\n");
    }
}
