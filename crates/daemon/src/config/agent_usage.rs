//! The operator's settings of the agent usage counts: `agent-usage.json` in the daemon's folder
//! (AIS-008, AIS-023, K8 Section 4). The operator writes it; the daemon only reads it, at start.
//!
//! ```json
//! { "off": true, "excludedFolders": ["/Users/me/private"], "excludedRepositories": ["github.com/me/secret"] }
//! ```

use std::io;
use std::path::{Path, PathBuf};

use daemon_core::agent_usage::is_repository;
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
    /// A session of one of these repositories (`host/owner/name`) is not counted, nor, when
    /// any is listed, a session whose repository name cannot be read.
    pub excluded_repositories: Vec<String>,
    /// Folders compare without letter case, as the file system of macOS does.
    pub fold_case: bool,
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
    #[error(
        "{SETTINGS_FILE} names a repository not written host/owner/name (no scheme, user or .git)"
    )]
    Repository,
}

impl UsageSettings {
    /// The settings of `home`; the default when the file is not there. Each excluded folder
    /// is kept as written and, when it exists, with its links resolved; on macOS folders
    /// compare without letter case.
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
        let mut settings = Self::parse(&read_limited(file, LIMIT)?)?;
        let resolved: Vec<PathBuf> = settings
            .excluded_folders
            .iter()
            .filter_map(|folder| std::fs::canonicalize(folder).ok())
            .collect();
        settings.excluded_folders.extend(resolved);
        settings.fold_case = cfg!(any(target_os = "macos", windows));
        Ok(settings)
    }

    /// The settings in a file's bytes. A member it does not know is refused, so a misspelt
    /// exclusion is never silently ignored.
    ///
    /// # Errors
    /// [`SettingsError`] for bytes that are not the form, a relative folder, a repository that
    /// is not written as the K8 `repository` is (it would never match), or a long list.
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
        let written_right = stored.excluded_repositories.iter().all(|name| {
            is_repository(&name.to_ascii_lowercase())
                && !Path::new(name)
                    .extension()
                    .is_some_and(|last| last.eq_ignore_ascii_case("git"))
        });
        if !written_right {
            return Err(SettingsError::Repository);
        }
        Ok(Self {
            off: stored.off,
            excluded_folders,
            excluded_repositories: stored.excluded_repositories,
            fold_case: false,
        })
    }

    /// Whether a session in `folders` (its working folder and the main folder of its
    /// repository) or of `repository` is excluded. Folders compare by whole components, so
    /// `/a/private` does not exclude `/a/private-not`, and without letter case when
    /// `fold_case`; repositories compare without case. When any repository is excluded, a
    /// session whose repository name is not known is excluded too (AIS-023).
    #[must_use]
    pub fn excludes(&self, folders: &[&Path], repository: Option<&str>) -> bool {
        let folder_excluded = folders.iter().any(|folder| {
            self.excluded_folders
                .iter()
                .any(|excluded| self.under(folder, excluded))
        });
        let repository_excluded = match repository {
            Some(name) => self
                .excluded_repositories
                .iter()
                .any(|excluded| excluded.eq_ignore_ascii_case(name)),
            None => !self.excluded_repositories.is_empty(),
        };
        folder_excluded || repository_excluded
    }

    fn under(&self, folder: &Path, excluded: &Path) -> bool {
        if !self.fold_case {
            return folder.starts_with(excluded);
        }
        let lower = |path: &Path| PathBuf::from(path.to_string_lossy().to_lowercase());
        lower(folder).starts_with(lower(excluded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ais_008_the_settings_read_back_and_exclude_by_folder_and_repository() {
        let folder = if cfg!(windows) {
            "C:/Users/me/private"
        } else {
            "/Users/me/private"
        };
        let document = serde_json::json!({"excludedFolders": [folder], "excludedRepositories": ["github.com/Me/Secret"]});
        let settings =
            UsageSettings::parse(&serde_json::to_vec(&document).unwrap()).expect("parses");
        assert!(!settings.off);
        let app = Some("github.com/me/app");
        let private = Path::new(if cfg!(windows) {
            "C:/Users/me/private/app"
        } else {
            "/Users/me/private/app"
        });
        assert!(settings.excludes(&[private], app));
        let near = Path::new(if cfg!(windows) {
            "C:/Users/me/private-not"
        } else {
            "/Users/me/private-not"
        });
        assert!(!settings.excludes(&[near], app), "whole components");
        let elsewhere = Path::new(if cfg!(windows) {
            "C:/tmp/wt"
        } else {
            "/tmp/wt"
        });
        let main = Path::new(if cfg!(windows) {
            "C:/Users/me/private"
        } else {
            "/Users/me/private"
        });
        assert!(
            settings.excludes(&[elsewhere, main], app),
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

    /// With a repository excluded, a session whose repository name cannot be read (a clone in
    /// a guarded folder, a remote that is not sent) may be that repository (AU-5 review).
    #[test]
    fn ais_023_an_unknown_repository_is_excluded_when_any_repository_is() {
        let listed = UsageSettings::parse(br#"{"excludedRepositories":["github.com/me/secret"]}"#)
            .expect("parses");
        let folder = Path::new(if cfg!(windows) {
            "C:/Users/me/Documents/secret"
        } else {
            "/Users/me/Documents/secret"
        });
        assert!(listed.excludes(&[folder], None));
        assert!(!UsageSettings::default().excludes(&[folder], None));
    }

    #[test]
    fn ais_008_a_repository_not_written_as_k8_writes_it_is_refused() {
        for name in [
            "https://github.com/me/app",
            "github.com/me/app.git",
            "git@github.com:me/app",
        ] {
            let text = format!(r#"{{"excludedRepositories":["{name}"]}}"#);
            let parsed = UsageSettings::parse(text.as_bytes());
            assert!(matches!(parsed, Err(SettingsError::Repository)), "{name}");
        }
        let upper = br#"{"excludedRepositories":["GitHub.com/Me/App"]}"#;
        assert!(
            UsageSettings::parse(upper).is_ok(),
            "letter case is not a form"
        );
    }

    /// An excluded folder written through a link (`/tmp` on macOS is `/private/tmp`) matches
    /// the folder a session records (AU-5 review).
    #[test]
    fn ais_008_an_excluded_folder_matches_with_its_links_resolved() {
        let dir = tempfile::tempdir().expect("a folder");
        let real = dir.path().join("real/secret");
        std::fs::create_dir_all(&real).expect("a folder");
        crate::test_links::symlink(dir.path().join("real"), dir.path().join("link"))
            .expect("a link");
        let written = dir.path().join("link/secret");
        let text = serde_json::to_vec(&serde_json::json!({"excludedFolders": [written]})).unwrap();
        std::fs::write(dir.path().join(SETTINGS_FILE), text).expect("written");
        let settings = UsageSettings::read(&Home::at(dir.path().to_owned())).expect("read");
        let recorded = std::fs::canonicalize(&real).expect("real").join("app");
        assert!(settings.excludes(&[recorded.as_path()], None));
        assert!(
            settings.excludes(&[written.join("app").as_path()], None),
            "as written"
        );
    }

    #[test]
    fn ais_008_folders_compare_without_letter_case_on_macos() {
        let settings = UsageSettings {
            excluded_folders: vec![PathBuf::from(if cfg!(windows) {
                "C:/Users/Me/Private"
            } else {
                "/Users/Me/Private"
            })],
            fold_case: true,
            ..UsageSettings::default()
        };
        assert!(settings.excludes(
            &[Path::new(if cfg!(windows) {
                "C:/users/me/private/app"
            } else {
                "/users/me/private/app"
            })],
            None
        ));
        assert!(!settings.excludes(
            &[Path::new(if cfg!(windows) {
                "C:/users/me/private-not"
            } else {
                "/users/me/private-not"
            })],
            None
        ));
        let exact = UsageSettings {
            fold_case: false,
            ..settings
        };
        assert!(!exact.excludes(
            &[Path::new(if cfg!(windows) {
                "C:/users/me/private/app"
            } else {
                "/users/me/private/app"
            })],
            None
        ));
    }
}
