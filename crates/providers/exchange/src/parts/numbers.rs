//! Numbers and commit hashes: the parts that name one thing in a repository.

use std::fmt;

use super::PartError;

/// The largest number a part may hold: 2^53, the largest whole number a JSON reader that
/// uses doubles keeps exactly.
const LARGEST: u64 = 1 << 53;

/// The number of an issue or a pull request, 1 to 2^53.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Number(u64);

impl Number {
    /// # Errors
    /// [`PartError::Number`] for 0 or a value above 2^53.
    pub const fn new(value: u64) -> Result<Self, PartError> {
        if value == 0 || value > LARGEST {
            Err(PartError::Number)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The ID of a workflow run, 1 to 2^53.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId(u64);

impl RunId {
    /// Reads decimal digits with no sign, no leading zero and nothing else.
    ///
    /// # Errors
    /// [`PartError::Number`] for anything else, 0 or a value above 2^53.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        digits(text).map(Self)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Decimal digits with no sign and no leading zero, 1 to 2^53.
pub(super) fn digits(text: &str) -> Result<u64, PartError> {
    let canonical = !text.is_empty()
        && text.len() <= 16
        && !text.starts_with('0')
        && text.bytes().all(|b| b.is_ascii_digit());
    if !canonical {
        return Err(PartError::Number);
    }
    let value = text.parse::<u64>().map_err(|_| PartError::Number)?;
    Number::new(value).map(Number::get)
}

/// A full commit hash: 40 hexadecimal digits, kept in lower case.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitSha(String);

impl CommitSha {
    /// # Errors
    /// [`PartError::Sha`] for anything but 40 hexadecimal digits.
    pub fn parse(text: &str) -> Result<Self, PartError> {
        if text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
            Ok(Self(text.to_ascii_lowercase()))
        } else {
            Err(PartError::Sha)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommitSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
