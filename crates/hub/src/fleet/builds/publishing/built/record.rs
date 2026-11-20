//! `anima-hub release record <target> <version> <folder>`: the signed release record of the
//! program files in a folder (ROL-003 to ROL-005; task U2). The job of the release workflow that
//! holds the updater key runs it (R-173). It writes `release-record.json` and its `.sig` beside
//! the files, and checks them with the public key before it ends (ROL-010).

use std::{collections::BTreeMap, path::Path};

use release_record::{Database, FORMAT, File, Program, RECORD_FILE, Record, SIGNATURE_FILE};

use super::command::{ReleaseError, files_of, open_key, setting};
use crate::fleet::builds::{model::Target, version::Version};

/// The release commit, 40 hexadecimal digits.
const COMMIT: &str = "ANIMA_RELEASE_COMMIT";
/// The lowest version that the release can replace (ROL-041).
const REPLACES_FROM: &str = "ANIMA_RELEASE_REPLACES_FROM";

/// Why no record was written. Names a file or a rule, never a key.
#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("{0} in the folder is not the file of a program of Anima")]
    NotAProgram(String),
    #[error("the folder holds no program file")]
    Empty,
    #[error("the release record breaks a rule, or its signature did not check")]
    Refused(#[from] release_record::SignError),
}

const fn target_of(target: Target) -> release_record::Target {
    match target {
        Target::LinuxX86_64 => release_record::Target::LinuxX8664,
        Target::DarwinUniversal => release_record::Target::DarwinUniversal,
    }
}

/// The text of a version in the form of the record.
fn version_of(text: &str, name: &'static str) -> Result<release_record::Version, ReleaseError> {
    text.parse().map_err(|_| ReleaseError::Setting(name))
}

/// Writes the signed record of the program files in `folder`.
pub(super) fn record(
    env: &BTreeMap<String, String>,
    target: Target,
    version: Version,
    folder: &Path,
) -> Result<String, ReleaseError> {
    let commit = setting(env, COMMIT)?.to_owned();
    let replaces_from = version_of(setting(env, REPLACES_FROM)?, REPLACES_FROM)?;
    let mut files = Vec::new();
    for file in files_of(folder)? {
        let program = Program::named(&file.name).ok_or(RecordError::NotAProgram(file.name))?;
        files.push(File::of(program, &file.bytes));
    }
    if files.is_empty() {
        return Err(RecordError::Empty.into());
    }
    files.sort_by_key(|file| file.program.name());
    let record = Record {
        format: FORMAT,
        version: version_of(&version.to_string(), "the version")?,
        commit,
        target: target_of(target),
        replaces_from,
        database: Database::Compatible,
        files,
    };
    let (key, signed_at) = open_key(env)?;
    let (bytes, signature) = key
        .sign_record(&record, signed_at)
        .map_err(RecordError::from)?;
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(folder.join(name), bytes).map_err(|error| ReleaseError::Folder(error.kind()))
    };
    write(RECORD_FILE, &bytes)?;
    write(SIGNATURE_FILE, signature.as_bytes())?;
    let programs: Vec<&str> = record.files.iter().map(|file| file.name.as_str()).collect();
    Ok(format!(
        "Recorded: {target} {version} with {}. {RECORD_FILE} and its signature are written.",
        programs.join(", ")
    ))
}
