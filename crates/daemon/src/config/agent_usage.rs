//! The operator's settings of the agent usage counts: `agent-usage.json` in the daemon's folder
//! (AIS-008, AIS-023, K8 Section 4). The operator writes it; the daemon only reads it, at start.
//!
//! ```json
//! { "off": true, "excludedFolders": ["/Users/me/private"], "excludedRepositories": ["github.com/me/secret"] }
//! ```

use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::home::Home;
use super::private_file::{LIMIT, read_limited};

/// The settings file in the daemon's folder.
pub const SETTINGS_FILE: &str = "agent-usage.json";

/// The most entries of each list.
const MAX_ENTRIES: usize = 256;

/// What the operator chose; the default reads every session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageSettings {
    /// No session file is read and no counts are sent.
    pub off: bool,
    /// A session whose working folder, or the main folder of its repository, is under one of
    /// these is not counted.
    pub excluded_folders: Vec<PathBuf>,
    /// A session of one of these repositories (`host/owner/name`) is not counted.
    pub excluded_repositories: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    #[serde(default)]
    off: bool,
    #[serde(default)]
    excluded_folders: Vec<String>,
    #[serde(default)]
    excluded_repositories: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{SETTINGS_FILE} cannot be read: {0}")]
    Io(#[from] io::Error),
    #[error(
        "{SETTINGS_FILE} is not JSON of its form (members off, excludedFolders, excludedRepositories)"
    )]
    Unreadable,
    #[error("{SETTINGS_FILE} names a folder that is not an absolute path")]
    RelativeFolder,
    #[error("{SETTINGS_FILE} has more than {MAX_ENTRIES} entries in a list")]
    TooMany,
}

impl UsageSettings {
    /// The settings of `home`; the default when the file is not there.
    ///
    /// # Errors
    /// [`SettingsError`] when the file cannot be read or parsed. The caller then turns the
    /// function off: an exclusion it cannot read must not let a session through.
    pub fn read(home: &Home) -> Result<Self, SettingsError> {
        let file = match std::fs::File::open(home.dir().join(SETTINGS_FILE)) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error.into()),
        };
        Self::parse(&read_limited(file, LIMIT)?)
    }

    /// The settings in a file's bytes. A member it does not know is refused, so a misspelt
    /// exclusion is never silently ignored.
    ///
    /// # Errors
    /// [`SettingsError`] for bytes that are not the form, a relative folder or a long list.
    pub fn parse(bytes: &[u8]) -> Result<Self, SettingsError> {
        let stored: Stored =
            serde_json::from_slice(bytes).map_err(|_| SettingsError::Unreadable)?;
        if stored.excluded_folders.len() > MAX_ENTRIES
            || stored.excluded_repositories.len() > MAX_ENTRIES
        {
            return Err(SettingsError::TooMany);
        }
        let excluded_folders = stored
            .excluded_folders
            .into_iter()
            .map(PathBuf::from)
            .map(|folder| folder.is_absolute().then_some(folder))
            .collect::<Option<Vec<_>>>()
            .ok_or(SettingsError::RelativeFolder)?;
        Ok(Self {
            off: stored.off,
            excluded_folders,
            excluded_repositories: stored.excluded_repositories,
        })
    }

    /// Whether a session in `folders` (its working folder and the main folder of its
    /// repository) or of `repository` is excluded. Folders compare by whole components, so
    /// `/a/private` does not exclude `/a/private-not`; repositories compare without case.
    #[must_use]
    pub fn excludes(&self, folders: &[&Path], repository: Option<&str>) -> bool {
        let folder_excluded = folders.iter().any(|folder| {
            self.excluded_folders
                .iter()
                .any(|excluded| folder.starts_with(excluded))
        });
        let repository_excluded = repository.is_some_and(|name| {
            self.excluded_repositories
                .iter()
                .any(|excluded| excluded.eq_ignore_ascii_case(name))
        });
        folder_excluded || repository_excluded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ais_008_the_settings_read_back_and_exclude_by_folder_and_repository() {
        let settings = UsageSettings::parse(
            br#"{"excludedFolders":["/Users/me/private"],"excludedRepositories":["github.com/Me/Secret"]}"#,
        )
        .expect("parses");
        assert!(!settings.off);
        let private = Path::new("/Users/me/private/app");
        assert!(settings.excludes(&[private], None));
        let near = Path::new("/Users/me/private-not");
        assert!(!settings.excludes(&[near], None), "whole components");
        let elsewhere = Path::new("/tmp/wt");
        let main = Path::new("/Users/me/private");
        assert!(
            settings.excludes(&[elsewhere, main], None),
            "a worktree of an excluded repository"
        );
        assert!(settings.excludes(&[elsewhere], Some("github.com/me/secret")));
        assert!(!settings.excludes(&[elsewhere], Some("github.com/me/app")));
        assert_eq!(
            UsageSettings::parse(b"{}").expect("empty"),
            UsageSettings::default()
        );
        assert!(UsageSettings::parse(br#"{"off":true}"#).expect("off").off);
    }

    #[test]
    fn ais_008_a_settings_file_that_cannot_be_trusted_is_refused() {
        assert!(matches!(
            UsageSettings::parse(br#"{"excludeFolders":["/x"]}"#),
            Err(SettingsError::Unreadable)
        ));
        assert!(matches!(
            UsageSettings::parse(br#"{"excludedFolders":["private"]}"#),
            Err(SettingsError::RelativeFolder)
        ));
        let many = format!(
            r#"{{"excludedRepositories":[{}]}}"#,
            vec!["\"a/b/c\""; 257].join(",")
        );
        assert!(matches!(
            UsageSettings::parse(many.as_bytes()),
            Err(SettingsError::TooMany)
        ));
        assert!(matches!(
            UsageSettings::parse(b"not json"),
            Err(SettingsError::Unreadable)
        ));
    }
}
