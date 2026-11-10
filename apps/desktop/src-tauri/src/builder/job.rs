//! The job fields the hub sends, read before any process starts (BLD-027, BLD-041, BLD-043). They
//! come from the web view, so each is parsed into a type that can only hold a safe value: it
//! becomes a process argument and a path part.

use super::error::BuilderError;

/// A commit hash: 40 lower-case hex digits, as the hub stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit(String);

impl Commit {
    pub fn parse(text: &str) -> Result<Self, BuilderError> {
        let hex = text.len() == 40
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        hex.then(|| Self(text.to_owned()))
            .ok_or(BuilderError::Commit)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A release version, `MAJOR.MINOR.PATCH`, each part 1 to 9 digits with no leading zero: the
/// same rule as the hub's (BLD-002), so the text is also a valid semver and a safe file name part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(String);

impl Version {
    pub fn parse(text: &str) -> Result<Self, BuilderError> {
        let part = |p: &str| {
            !p.is_empty()
                && p.len() <= 9
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p == "0" || !p.starts_with('0'))
        };
        let parts: Vec<&str> = text.split('.').collect();
        (parts.len() == 3 && parts.iter().all(|p| part(p)))
            .then(|| Self(text.to_owned()))
            .ok_or(BuilderError::Version)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// K1 `BuildTarget`: what a job builds (BLD-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    LinuxX86_64,
    DarwinUniversal,
}

impl Target {
    pub fn parse(text: &str) -> Result<Self, BuilderError> {
        match text {
            "LINUX_X86_64" => Ok(Self::LinuxX86_64),
            "DARWIN_UNIVERSAL" => Ok(Self::DarwinUniversal),
            _ => Err(BuilderError::Target),
        }
    }

    /// Whether this computer can build the target: Linux on `x86_64`, or macOS on `aarch64`.
    pub fn buildable_on(self, os: &str, arch: &str) -> bool {
        match self {
            Self::LinuxX86_64 => os == "linux" && arch == "x86_64",
            Self::DarwinUniversal => os == "macos" && arch == "aarch64",
        }
    }

    /// The folder of the target's files in the release bucket (BLD-060).
    pub const fn folder(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "linux-x86_64",
            Self::DarwinUniversal => "darwin-universal",
        }
    }

    /// The target this computer can build, if any.
    pub fn here() -> Result<Self, BuilderError> {
        [Self::LinuxX86_64, Self::DarwinUniversal]
            .into_iter()
            .find(|target| target.buildable_on(std::env::consts::OS, std::env::consts::ARCH))
            .ok_or(BuilderError::Platform)
    }
}
