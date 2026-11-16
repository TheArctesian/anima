//! Scratch refs: the git refs under `refs/anima/wip/` that hold a snapshot of a working tree
//! (JOB-016), and the remote names an operator may give. Pure: no git, no clock.

use serde::Serialize;

use super::SnapshotError;

/// Every scratch ref starts with this; nothing outside it is ever pushed or deleted.
pub const PREFIX: &str = "refs/anima/wip/";

/// How many hex characters of the commit hash name a snapshot.
const ID_LENGTH: usize = 16;

/// A ref under [`PREFIX`] whose last part is a safe single name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ScratchRef(String);

impl ScratchRef {
    /// The ref for `commit`: its first 16 hex characters, so the same commit always gets the
    /// same ref.
    ///
    /// # Errors
    /// [`SnapshotError::Git`] when `commit` is not a hex hash git could have printed.
    pub fn for_commit(commit: &str) -> Result<Self, SnapshotError> {
        let id = commit.get(..ID_LENGTH).filter(|_| is_hash(commit));
        id.map(|id| Self(format!("{PREFIX}{id}")))
            .ok_or_else(|| SnapshotError::Git {
                step: "commit-tree".to_owned(),
                reason: "it did not print a commit hash".to_owned(),
            })
    }

    /// What the operator typed: a full ref under [`PREFIX`], or the name after it.
    ///
    /// # Errors
    /// [`SnapshotError::NotScratchRef`] for any other ref or an unsafe name.
    pub fn parse(text: &str) -> Result<Self, SnapshotError> {
        let name = if text.starts_with("refs/") {
            text.strip_prefix(PREFIX)
        } else {
            Some(text)
        };
        name.filter(|name| is_safe_name(name))
            .map(|name| Self(format!("{PREFIX}{name}")))
            .ok_or_else(|| SnapshotError::NotScratchRef {
                text: text.to_owned(),
            })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A remote name as the operator gave it; it must still be one `git remote` lists.
///
/// # Errors
/// [`SnapshotError::BadRemote`] for an empty name, one that starts with `-` (git would read it
/// as an option) or one with a space or control character.
pub fn checked_remote(name: &str) -> Result<&str, SnapshotError> {
    let plain = !name.is_empty()
        && !name.starts_with('-')
        && !name.chars().any(|c| c.is_whitespace() || c.is_control());
    if plain {
        Ok(name)
    } else {
        Err(SnapshotError::BadRemote {
            name: name.to_owned(),
        })
    }
}

/// The line a second machine runs to get the snapshot.
#[must_use]
pub fn fetch_line(remote: &str, scratch: &ScratchRef) -> String {
    format!("git fetch {remote} {}", scratch.as_str())
}

/// At least 16 hex characters, as SHA-1 (40) and SHA-256 (64) hashes are.
#[must_use]
pub fn is_hash(text: &str) -> bool {
    text.len() >= ID_LENGTH && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One ref part git accepts and no shell or option can misread: letters, digits, `.`, `_` and
/// `-`; not starting with `.` or `-`; no `..`; not ending in `.lock` in any case.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !name.starts_with(['.', '-'])
        && !name.contains("..")
        && !std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn job_016_the_ref_is_the_first_16_hex_characters_of_the_commit() {
        let scratch = ScratchRef::for_commit(COMMIT).unwrap();
        assert_eq!(scratch.as_str(), "refs/anima/wip/0123456789abcdef");
        assert_eq!(ScratchRef::for_commit(COMMIT).unwrap(), scratch);
        assert!(ScratchRef::for_commit("0123").is_err());
        assert!(ScratchRef::for_commit("refs/heads/main/0123456789").is_err());
    }

    #[test]
    fn job_017_a_ref_or_an_id_names_a_scratch_ref() {
        let full = ScratchRef::parse("refs/anima/wip/0123456789abcdef").unwrap();
        let short = ScratchRef::parse("0123456789abcdef").unwrap();
        assert_eq!(full, short);
        assert_eq!(full.as_str(), "refs/anima/wip/0123456789abcdef");
    }

    #[test]
    fn job_017_a_ref_outside_the_scratch_space_is_refused() {
        for text in [
            "refs/heads/main",
            "refs/tags/v1",
            "refs/anima/wipe/x",
            "refs/anima/wip/",
            "refs/anima/wip/a/b",
            "refs/anima/wip/../heads/main",
            "-delete",
            "--force",
            ".hidden",
            "a..b",
            "x.lock",
            "a b",
            "a:b",
            "+x",
            "",
        ] {
            assert!(
                matches!(
                    ScratchRef::parse(text),
                    Err(SnapshotError::NotScratchRef { .. })
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn job_016_a_remote_that_reads_as_an_option_is_refused() {
        assert_eq!(checked_remote("origin").unwrap(), "origin");
        for name in ["", "-u", "--receive-pack=x", "a b", "a\nb"] {
            assert!(checked_remote(name).is_err(), "{name:?}");
        }
    }

    #[test]
    fn job_016_the_fetch_line_names_the_remote_and_the_ref() {
        let scratch = ScratchRef::for_commit(COMMIT).unwrap();
        assert_eq!(
            fetch_line("origin", &scratch),
            "git fetch origin refs/anima/wip/0123456789abcdef"
        );
    }
}
