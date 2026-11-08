//! A release version, `MAJOR.MINOR.PATCH` (BLD-002): its parser, its order and the next one.
//!
//! Pure. Only the exact text the parser accepts reaches the store, so two spellings never name
//! one version.

use std::{fmt, str::FromStr};

/// The largest number one part may have: nine digits, as the stored check allows.
const PART_MAX: u32 = 999_999_999;

/// A version of the desktop app. Ordered part by part, so `0.1.10 > 0.1.9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

/// The text is not `MAJOR.MINOR.PATCH` with each part a number of 1 to 9 digits and no
/// leading zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a version is three numbers with dots between, such as 0.1.12")]
pub struct VersionError;

impl Version {
    #[cfg(test)]
    pub(super) const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The version with the patch number plus 1, which the release form proposes (Section
    /// 3.10). `None` when the patch is at its largest.
    #[must_use]
    pub fn next_patch(self) -> Option<Self> {
        let patch = self.patch.checked_add(1).filter(|p| *p <= PART_MAX)?;
        Some(Self { patch, ..self })
    }
}

/// One part: 1 to 9 ASCII digits, no leading zero unless the part is `0`.
fn part(text: &str) -> Result<u32, VersionError> {
    let canonical = !text.is_empty()
        && text.len() <= 9
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if !canonical {
        return Err(VersionError);
    }
    text.parse().map_err(|_| VersionError)
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(VersionError);
        };
        Ok(Self {
            major: part(major)?,
            minor: part(minor)?,
            patch: part(patch)?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
