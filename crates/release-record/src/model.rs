//! The fields of a release record (ROL-003). An unknown field is refused: a record that needs a
//! new field has a new [`FORMAT`], which an older reader refuses as a whole.

use serde::{Deserialize, Serialize};

use crate::version::Version;

/// The format of the record that this crate writes and reads.
pub const FORMAT: u32 = 1;

/// A target of a release (OD-ROL-8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    #[serde(rename = "linux-x86_64")]
    LinuxX8664,
    #[serde(rename = "darwin-universal")]
    DarwinUniversal,
}

/// A program of Anima that a record can name (ROL-001, ROL-013). The desktop application is
/// not one: it updates by its own feed (OD-ROL-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Program {
    #[serde(rename = "anima-daemon")]
    Daemon,
    #[serde(rename = "anima")]
    Command,
    #[serde(rename = "anima-hub")]
    Hub,
    #[serde(rename = "anima-gateway")]
    Gateway,
    #[serde(rename = "anima-link")]
    Link,
}

/// What the hub of the release before can do with the database of this release (ROL-034,
/// ROL-036).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Database {
    /// The hub of the release before can use it: the migrations only add (ROL-034).
    Compatible,
    /// It cannot; the way back is the restore of a copy. A rollout needs a second confirmation
    /// and an automatic rollout never takes it (ROL-036).
    RestoreNeeded,
}

/// One program file of the release, under `<target>/<version>/<name>` (ROL-005).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub name: String,
    pub program: Program,
    pub size: u64,
    /// The SHA-256 of the file, 64 hexadecimal digits in lower case.
    pub sha256: String,
}

/// The release record of one target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub format: u32,
    pub version: Version,
    /// The release commit, 40 hexadecimal digits in lower case.
    pub commit: String,
    pub target: Target,
    /// The lowest version that this release can replace (ROL-041).
    pub replaces_from: Version,
    pub database: Database,
    /// One file for each program, at most one for a program (ROL-013).
    pub files: Vec<File>,
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// A file name of the release host: 1 to 100 of `A-Z a-z 0-9 . _ -`, not `.` first.
fn is_file_name(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

impl Program {
    /// Each program, in the order of ROL-001.
    pub const ALL: [Self; 5] = [
        Self::Daemon,
        Self::Command,
        Self::Hub,
        Self::Gateway,
        Self::Link,
    ];

    /// The name of the program, which is also the name of its file in a release folder.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Daemon => "anima-daemon",
            Self::Command => "anima",
            Self::Hub => "anima-hub",
            Self::Gateway => "anima-gateway",
            Self::Link => "anima-link",
        }
    }

    /// The program with the name `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|program| program.name() == name)
    }
}

impl File {
    /// The entry of `program` for the bytes of its file, under the name of the program.
    #[must_use]
    pub fn of(program: Program, bytes: &[u8]) -> Self {
        use sha2::Digest as _;
        Self {
            name: program.name().to_owned(),
            program,
            size: bytes.len() as u64,
            sha256: crate::check::to_hex(&sha2::Sha256::digest(bytes)),
        }
    }
}

impl Record {
    /// Whether the fields obey the rules that the types do not hold: the format, the commit,
    /// the names, the sizes and hashes, one file for each program, no two files of one name,
    /// and a lowest replaceable version that is not above the version.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let files_valid = self.files.iter().all(|file| {
            is_file_name(&file.name)
                && file.size <= crate::MAX_FILE_BYTES
                && is_hex(&file.sha256, 64)
        });
        let unique = self.files.iter().enumerate().all(|(index, file)| {
            self.files[..index]
                .iter()
                .all(|before| before.program != file.program && before.name != file.name)
        });
        self.format == FORMAT
            && is_hex(&self.commit, 40)
            && self.replaces_from <= self.version
            && !self.files.is_empty()
            && files_valid
            && unique
    }

    /// The file of `program`, if the record names one.
    #[must_use]
    pub fn file_of(&self, program: Program) -> Option<&File> {
        self.files.iter().find(|file| file.program == program)
    }
}
