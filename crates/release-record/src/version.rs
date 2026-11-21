//! A release version, `MAJOR.MINOR.PATCH`, with the rule of BLD-002 and its order.

use std::{fmt, str::FromStr};

/// A version, ordered part by part, so `0.1.10 > 0.1.9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

/// The text is not `MAJOR.MINOR.PATCH` with each part a number of 1 to 9 digits and no leading
/// zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a version is three numbers with dots between, such as 0.1.12")]
pub struct VersionError;

fn part(text: &str) -> Result<u32, VersionError> {
    let canonical = !text.is_empty()
        && text.len() <= 9
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if canonical {
        text.parse().map_err(|_| VersionError)
    } else {
        Err(VersionError)
    }
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

impl serde::Serialize for Version {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <&str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}
