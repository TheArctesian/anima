//! Reads a channel document from a file or standard input (CMD-025). The document holds the
//! channel's secrets, so the bytes are wiped when they drop and an error says where the
//! document is wrong, never what it holds (CMD-014, LOG-007).

use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::failure::Failure;
use crate::invocation::Invocation;

/// The largest document read.
const MAX_DOCUMENT: usize = 1024 * 1024;

/// The document in `from` (`-` for standard input) as `T`.
///
/// # Errors
/// [`Failure::Usage`] for `-` at a terminal; [`Failure::Input`] for an unreadable or invalid
/// document.
pub fn read<T: DeserializeOwned>(invocation: &Invocation, from: &Path) -> Result<T, Failure> {
    let what = "the channel document".to_owned();
    let stdin = from.as_os_str() == "-";
    if stdin && invocation.stdin_terminal() {
        return Err(Failure::Usage {
            message: "The channel document comes from standard input: pipe it in, or pass --from-file <FILE>."
                .to_owned(),
            hint: None,
        });
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_DOCUMENT + 1));
    let limit = MAX_DOCUMENT as u64 + 1;
    let read = if stdin {
        std::io::stdin().lock().take(limit).read_to_end(&mut bytes)
    } else {
        File::open(from).and_then(|file| file.take(limit).read_to_end(&mut bytes))
    };
    read.map_err(|error| Failure::Input {
        what: from.display().to_string(),
        reason: error.kind().to_string(),
    })?;
    if bytes.len() > MAX_DOCUMENT {
        return Err(Failure::Input {
            what,
            reason: format!("it is larger than {MAX_DOCUMENT} bytes"),
        });
    }
    parse(&bytes)
}

/// `bytes` as `T`. The error names the line and column, not the serde message, which can
/// quote a value.
fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Failure> {
    serde_json::from_slice(bytes).map_err(|error| Failure::Input {
        what: "the channel document".to_owned(),
        reason: format!(
            "it is {} at line {}, column {} (anima notify channel create --help shows its shape)",
            match error.classify() {
                serde_json::error::Category::Data => "not a channel",
                _ => "not valid JSON",
            },
            error.line(),
            error.column()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_014_an_invalid_document_is_reported_without_quoting_it() {
        let secret = "hunter2-the-smtp-password";
        let wrong_type = format!(r#"{{"name": "{secret}", "n": 3}}"#);
        let error = parse::<Vec<u32>>(wrong_type.as_bytes()).expect_err("wrong shape");
        assert!(!error.to_string().contains(secret), "{error}");
        let broken = format!(r#"{{"password": "{secret}" "#);
        let error = parse::<serde_json::Value>(broken.as_bytes()).expect_err("broken");
        assert!(!error.to_string().contains(secret), "{error}");
        assert!(error.to_string().contains("line 1"), "{error}");
    }
}
