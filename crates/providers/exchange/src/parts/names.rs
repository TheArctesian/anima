//! Names: a repository, a workflow, a branch, a status context and a status address.

use std::fmt;

use super::numbers::digits;
use super::{PartError, is_plain_segment};

/// The longest owner login and repository name the provider allows.
const OWNER_LONGEST: usize = 39;
const NAME_LONGEST: usize = 100;
/// The longest branch name and workflow file name, in bytes.
const BRANCH_LONGEST: usize = 255;
const WORKFLOW_LONGEST: usize = 255;
/// The longest status address, in bytes.
const TARGET_URL_LONGEST: usize = 2048;
/// Every status context Anima writes starts with this (PRO-010).
const CONTEXT_PREFIX: &str = "anima/";
const CONTEXT_LONGEST: usize = 40;

/// A repository as `owner/name`: two path segments of letters, digits, `.`, `-` and `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RepositoryName {
    owner: String,
    name: String,
}

impl RepositoryName {
    /// Reads `owner/name`.
    ///
    /// # Errors
    /// [`PartError::Repository`] when either half is empty, `.`, `..`, too long or holds any
    /// other character, or there is not exactly one `/`.
    pub fn parse(full_name: &str) -> Result<Self, PartError> {
        let (owner, name) = full_name.split_once('/').ok_or(PartError::Repository)?;
        let fits =
            |part: &str, longest: usize| part.len() <= longest && is_plain_segment(part.as_bytes());
        if fits(owner, OWNER_LONGEST) && fits(name, NAME_LONGEST) {
            Ok(Self {
                owner: owner.to_owned(),
                name: name.to_owned(),
            })
        } else {
            Err(PartError::Repository)
        }
    }

    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// A workflow: its number, or its file name such as `build.yml`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowId(String);

impl WorkflowId {
    /// # Errors
    /// [`PartError::Workflow`] for anything but a number from 1 to 2^53 or a file name of
    /// letters, digits, `.`, `-` and `_` ending `.yml` or `.yaml`.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        let number = digits(text).is_ok();
        let stem = text
            .strip_suffix(".yml")
            .or_else(|| text.strip_suffix(".yaml"))
            .filter(|stem| !stem.is_empty());
        let file =
            stem.is_some() && text.len() <= WORKFLOW_LONGEST && is_plain_segment(text.as_bytes());
        if number || file {
            Ok(Self(text.to_owned()))
        } else {
            Err(PartError::Workflow)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A branch name git allows (`git check-ref-format --branch`), 1 to 255 bytes. It may hold `/`,
/// so a provider that puts it in a path encodes it; it never holds a `.` or `..` segment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BranchName(String);

impl BranchName {
    /// # Errors
    /// [`PartError::Branch`] for a name git refuses or one longer than 255 bytes.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        if is_git_branch(text) {
            Ok(Self(text.to_owned()))
        } else {
            Err(PartError::Branch)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BranchName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The rules of `git check-ref-format` for a branch, with a length cap.
fn is_git_branch(text: &str) -> bool {
    let bad_byte = |b: u8| {
        b.is_ascii_control() || matches!(b, b' ' | b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
    };
    let bad_segment = |segment: &str| {
        segment.is_empty() || segment.starts_with('.') || segment.as_bytes().ends_with(b".lock")
    };
    !text.is_empty()
        && text.len() <= BRANCH_LONGEST
        && text != "@"
        && !text.starts_with('-')
        && !text.ends_with('.')
        && !text.contains("..")
        && !text.contains("@{")
        && !text.bytes().any(bad_byte)
        && !text.split('/').any(bad_segment)
}

/// A commit status context under `anima/`: Anima never writes another system's context.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatusContext(String);

impl StatusContext {
    /// # Errors
    /// [`PartError::StatusContext`] for anything but `anima/` then 1 to 40 of `a-z`, `0-9`
    /// and `-`.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        let rest = text
            .strip_prefix(CONTEXT_PREFIX)
            .ok_or(PartError::StatusContext)?;
        let plain = rest
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !rest.is_empty() && rest.len() <= CONTEXT_LONGEST && plain {
            Ok(Self(text.to_owned()))
        } else {
            Err(PartError::StatusContext)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The address a commit status links to: always a page below the hub's public origin.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TargetUrl(String);

impl TargetUrl {
    /// Checks that `url` is `origin` followed by `/` and a path, with no space, backslash or
    /// control character, at most 2,048 bytes. `origin` is the hub's public origin, such as
    /// `https://hub.example`, with no trailing slash.
    ///
    /// # Errors
    /// [`PartError::TargetUrl`] when `url` is not below `origin`, or `origin` is not an
    /// `http` or `https` origin.
    pub fn parse(url: &str, origin: &str) -> Result<Self, PartError> {
        let host = origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"))
            .ok_or(PartError::TargetUrl)?;
        let origin_is_plain = !host.is_empty() && !host.contains(['/', '@', '?', '#', '\\']);
        let below = url
            .strip_prefix(origin)
            .is_some_and(|path| path.starts_with('/') && !path.starts_with("//"));
        let clean = !url
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'\\');
        if origin_is_plain && below && clean && url.len() <= TARGET_URL_LONGEST {
            Ok(Self(url.to_owned()))
        } else {
            Err(PartError::TargetUrl)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
