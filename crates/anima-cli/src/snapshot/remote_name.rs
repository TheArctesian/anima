//! The remote name an operator gives with `--remote`, checked before git sees it. Pure.

use super::SnapshotError;

/// `name` when git can only read it as a plain remote name; it must still be one that
/// `git remote` lists.
///
/// # Errors
/// [`SnapshotError::BadRemote`] for an empty name, one that starts with `-` (git would read it
/// as an option), one with a space or control character, or one with a glob character (in
/// `--remotes=<name>` it would match the refs of other remotes too).
pub fn checked(name: &str) -> Result<&str, SnapshotError> {
    let plain = !name.is_empty()
        && !name.starts_with('-')
        && !name
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '*' | '?' | '[' | '\\'));
    if plain {
        Ok(name)
    } else {
        Err(SnapshotError::BadRemote {
            name: name.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_017_a_remote_that_reads_as_an_option_or_a_pattern_is_refused() {
        assert_eq!(checked("origin").unwrap(), "origin");
        assert_eq!(checked("my-forge.2").unwrap(), "my-forge.2");
        for name in [
            "",
            "-u",
            "--receive-pack=x",
            "a b",
            "a\nb",
            "o*",
            "orig?n",
            "[o]rigin",
            "a\\b",
        ] {
            assert!(checked(name).is_err(), "{name:?}");
        }
    }
}
