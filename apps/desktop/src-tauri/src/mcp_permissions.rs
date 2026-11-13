//! The Settings panel's view of what `anima mcp` may do for one hub (MCP-018). The panel reads
//! and changes the same file the command line does, through the command line's own library
//! functions: this module adds the origin check, the folder and the error codes, nothing else.
//! No token is read, and the `anima` program is not started.

use std::path::{Path, PathBuf};

use anima_cli::contexts::config::ConfigError;
use anima_cli::environment::process_config_dir;
use anima_cli::failure::Failure;
use anima_cli::hub::Origin;
use anima_cli::mcp::permission_change::{Change, Shown, edit, read};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri_plugin_log::log;

/// Why the permissions were not read or saved. Carries no path and no file text (LOG-007).
///
/// - `invalid_hub`: the hub address is not an origin.
/// - `no_folder`: the computer has no configuration folder.
/// - `unreadable`: the file cannot be read or is not a valid configuration.
/// - `unwritable`: the file cannot be written.
/// - `invalid_change`: the change names a tool or tool set that does not exist.
/// - `failed`: anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PermissionsError {
    #[error("the hub address is not valid")]
    InvalidHub,
    #[error("this computer has no configuration folder")]
    NoFolder,
    #[error("the permissions file cannot be read")]
    Unreadable,
    #[error("the permissions file cannot be written")]
    Unwritable,
    #[error("the change names a tool that does not exist")]
    InvalidChange,
    #[error("the permissions could not be changed")]
    Failed,
}

impl PermissionsError {
    /// Stable code the client switches on.
    fn code(self) -> &'static str {
        match self {
            Self::InvalidHub => "invalid_hub",
            Self::NoFolder => "no_folder",
            Self::Unreadable => "unreadable",
            Self::Unwritable => "unwritable",
            Self::InvalidChange => "invalid_change",
            Self::Failed => "failed",
        }
    }

    fn from_failure(failure: &Failure) -> Self {
        match failure {
            Failure::Config(ConfigError::NoFolder) => Self::NoFolder,
            Failure::Config(ConfigError::Read { .. } | ConfigError::Invalid { .. }) => {
                Self::Unreadable
            }
            Failure::Config(ConfigError::Write { .. }) => Self::Unwritable,
            Failure::Usage { .. } => Self::InvalidChange,
            _ => Self::Failed,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for PermissionsError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("PermissionsError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

fn origin(text: &str) -> Result<Origin, PermissionsError> {
    Origin::parse(text).map_err(|_| PermissionsError::InvalidHub)
}

fn folder(dir: Option<&Path>) -> Result<&Path, PermissionsError> {
    dir.ok_or(PermissionsError::NoFolder)
}

fn read_in(dir: Option<&Path>, hub_origin: &str) -> Result<Shown, PermissionsError> {
    read(folder(dir)?, &origin(hub_origin)?).map_err(|failure| match failure {
        // A saved name that does not exist: the file is unusable as it stands.
        Failure::Usage { .. } => PermissionsError::Unreadable,
        other => PermissionsError::from_failure(&other),
    })
}

fn write_in(
    dir: Option<&Path>,
    hub_origin: &str,
    change: &Change,
) -> Result<Shown, PermissionsError> {
    edit(folder(dir)?, &origin(hub_origin)?, change)
        .map_err(|failure| PermissionsError::from_failure(&failure))
}

/// Runs a blocking file job off the async runtime and logs only its code.
async fn run<T: Send + 'static>(
    job: impl FnOnce(Option<PathBuf>) -> Result<T, PermissionsError> + Send + 'static,
) -> Result<T, PermissionsError> {
    let done = tauri::async_runtime::spawn_blocking(move || job(process_config_dir()))
        .await
        .map_err(|_| PermissionsError::Failed)
        .flatten();
    if let Err(error) = &done {
        log::warn!(
            "the MCP permissions were not read or saved: {}",
            error.code()
        );
    }
    done
}

/// Each MCP tool with its saved state for `hub_origin`: what `anima mcp permissions show --json` prints.
#[tauri::command]
pub async fn mcp_permissions_read(hub_origin: String) -> Result<Shown, PermissionsError> {
    run(move |dir| read_in(dir.as_deref(), &hub_origin)).await
}

/// Applies one change to the saved states for `hub_origin`, saves the file, and answers with the new states.
#[tauri::command]
pub async fn mcp_permissions_write(
    hub_origin: String,
    change: Change,
) -> Result<Shown, PermissionsError> {
    run(move |dir| write_in(dir.as_deref(), &hub_origin, &change)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUB: &str = "https://hub.example.com";

    fn state_of(shown: &Shown, tool: &str) -> String {
        let json = serde_json::to_value(shown).expect("serialises");
        json["tools"]
            .as_array()
            .and_then(|tools| tools.iter().find(|row| row["tool"] == tool))
            .and_then(|row| row["state"].as_str())
            .unwrap_or("missing")
            .to_owned()
    }

    #[test]
    fn mcp_018_a_write_is_read_back_from_the_same_file_the_command_line_uses() {
        let dir = tempfile::tempdir().expect("a folder");
        let change: Change = serde_json::from_str(
            r#"{"type":"choose","names":["anima_check_update"],"state":"allow"}"#,
        )
        .expect("a change");
        let written = write_in(Some(dir.path()), HUB, &change).expect("written");
        assert_eq!(state_of(&written, "anima_check_update"), "allow");
        let read = read_in(Some(dir.path()), HUB).expect("read");
        assert_eq!(state_of(&read, "anima_check_update"), "allow");
        assert_eq!(state_of(&read, "anima_check_delete"), "off");
        assert!(dir.path().join("config.json").is_file());
    }

    #[test]
    fn mcp_018_a_hub_address_that_is_not_an_origin_is_refused_before_any_file_is_touched() {
        let dir = tempfile::tempdir().expect("a folder");
        for text in [
            "",
            "hub.example.com",
            "http://hub.example.com",
            "https://hub.example.com/x",
        ] {
            let error = read_in(Some(dir.path()), text).expect_err("refused");
            assert_eq!(error, PermissionsError::InvalidHub, "{text}");
        }
    }

    #[test]
    fn mcp_018_errors_carry_a_code_and_fixed_text_with_no_path() {
        let dir = tempfile::tempdir().expect("a folder");
        std::fs::write(dir.path().join("config.json"), "not json").expect("written");
        let error = read_in(Some(dir.path()), HUB).expect_err("unreadable");
        assert_eq!(error, PermissionsError::Unreadable);
        let text = serde_json::to_string(&error).expect("serialises");
        assert!(text.contains("unreadable"), "{text}");
        assert!(
            !text.contains(dir.path().to_string_lossy().as_ref()),
            "{text}"
        );
        assert_eq!(
            read_in(None, HUB).expect_err("no folder"),
            PermissionsError::NoFolder
        );
    }

    #[test]
    fn mcp_018_a_tool_that_does_not_exist_is_an_invalid_change() {
        let dir = tempfile::tempdir().expect("a folder");
        let change: Change =
            serde_json::from_str(r#"{"type":"choose","names":["nope"],"state":"allow"}"#)
                .expect("a change");
        let error = write_in(Some(dir.path()), HUB, &change).expect_err("refused");
        assert_eq!(error, PermissionsError::InvalidChange);
    }
}
