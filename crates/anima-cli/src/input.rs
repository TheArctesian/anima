//! A document the operator gives as a file or as standard input (`-`): read up to a size cap,
//! and parsed as JSON with one failure for every command that takes one (CMD-025, CMD-022).
//! Documents can hold secrets, so the bytes are wiped when they drop and a failure says where
//! the document is wrong, never what it holds (CMD-014, LOG-007).

use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::failure::{Failure, Hint};
use crate::prompt::Prompter;

/// The largest JSON document read.
pub const MAX_DOCUMENT: usize = 1024 * 1024;

/// The bytes of `from` (`-` for standard input), up to `limit`. `what` names the document in
/// failures. Standard input at a terminal is announced, or refused under `--no-input`, so the
/// command never waits in silence.
///
/// # Errors
/// [`Failure::Input`] when it cannot be read or is larger than `limit`; [`Failure::Usage`] for
/// a terminal under `--no-input`.
pub fn read(
    prompter: Prompter,
    from: &Path,
    what: &str,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, Failure> {
    let stdin = from.as_os_str() == "-";
    if stdin {
        prompter.reading_stdin(what)?;
    }
    let failed = |reason: String| Failure::Input {
        what: what.to_owned(),
        reason,
    };
    let mut bytes = Zeroizing::new(Vec::new());
    let cap = limit as u64 + 1;
    let read = if stdin {
        std::io::stdin().lock().take(cap).read_to_end(&mut bytes)
    } else {
        File::open(from).and_then(|file| file.take(cap).read_to_end(&mut bytes))
    };
    read.map_err(|error| failed(format!("{}: {}", from.display(), error.kind())))?;
    if bytes.len() > limit {
        return Err(failed(format!("it is larger than {limit} bytes")));
    }
    Ok(bytes)
}

/// `bytes` as JSON of type `T`. The failure is a usage failure (exit 2) naming the line and
/// column, not the serde message, which can quote a value; `help` is the command that shows
/// the document's shape.
///
/// # Errors
/// [`Failure::Usage`] when `bytes` is not JSON or does not fit `T`.
pub fn json<T: DeserializeOwned>(bytes: &[u8], what: &str, help: &str) -> Result<T, Failure> {
    serde_json::from_slice(bytes).map_err(|error| {
        let problem = match error.classify() {
            serde_json::error::Category::Data => "has a value that does not fit",
            _ => "is not valid JSON",
        };
        let mut words = what.chars();
        let first: String = words
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        Failure::Usage {
            message: format!(
                "{first}{} {problem} (line {}, column {}).",
                words.as_str(),
                error.line(),
                error.column()
            ),
            hint: Some(Hint::run(help)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_022_bad_json_is_a_usage_failure_that_never_quotes_the_document() {
        let secret = "hunter2-the-smtp-password";
        let wrong_type = format!(r#"{{"name": "{secret}", "n": 3}}"#);
        let error = json::<Vec<u32>>(wrong_type.as_bytes(), "the document", "anima api --help")
            .expect_err("wrong shape");
        assert!(!error.to_string().contains(secret), "{error}");
        assert_eq!(error.exit_code(), 2);
        let broken = format!(r#"{{"password": "{secret}" "#);
        let error =
            json::<serde_json::Value>(broken.as_bytes(), "the document", "anima api --help")
                .expect_err("broken");
        assert!(!error.to_string().contains(secret), "{error}");
        assert!(error.to_string().contains("line 1"), "{error}");
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn cmd_025_a_file_over_the_limit_is_refused_and_one_under_it_is_read() {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("document.json");
        std::fs::write(&path, "12345").expect("a file");
        let prompter = Prompter::new(false, false, false);
        let bytes = read(prompter, &path, "the document", 5).expect("fits");
        assert_eq!(bytes.as_slice(), b"12345");
        let failure = read(prompter, &path, "the document", 4).expect_err("too large");
        assert!(
            failure.to_string().contains("larger than 4 bytes"),
            "{failure}"
        );
    }
}
