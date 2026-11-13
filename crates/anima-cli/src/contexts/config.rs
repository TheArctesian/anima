//! `config.json` in the configuration folder (CMD-015): the named contexts and the current one.
//! It holds no secret; tokens live in the key store.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hub::Origin;
use crate::hub::origin::OriginError;
use crate::private_file::{self, LIMIT, PrivateFileError};

const FILE_NAME: &str = "config.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The context used when no `--context`, `--hub` or `ANIMA_HUB` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    #[serde(default)]
    pub contexts: BTreeMap<String, Context>,
    /// The saved `anima mcp` permissions of each hub, by hub origin (MCP-002).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mcp: BTreeMap<String, McpPermissions>,
}

/// The state (`allow`, `ask` or `off`) of each `anima mcp` tool for one hub: the operator's
/// explicit choices by tool name, and a default for each kind (`anima mcp permissions set`,
/// MCP-002, MCP-012). A tool with no entry takes its kind's default; a kind with no default
/// takes the built-in one (read `allow`, change `off`). Names and states only; no secret.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPermissions {
    /// The default of the read tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<String>,
    /// The default of the change tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    /// Explicit choices, by tool name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<String, String>,
    /// The form before MCP-012: the sets offered (empty: all). Read once and turned into
    /// explicit choices; never written.
    #[serde(default, skip_serializing)]
    pub toolsets: Vec<String>,
    /// The form before MCP-012: the sets whose change tools were offered. As `toolsets`.
    #[serde(default, skip_serializing)]
    pub write: Vec<String>,
}

/// One named hub.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub hub: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("No configuration folder: set HOME or XDG_CONFIG_HOME.")]
    NoFolder,
    #[error("Could not read {path}: {reason}.")]
    Read { path: PathBuf, reason: String },
    #[error("{path} is not a valid configuration file; fix or delete it.")]
    Invalid { path: PathBuf },
    #[error("Could not write {path}: {source}.")]
    Write {
        path: PathBuf,
        source: PrivateFileError,
    },
    #[error("The context {name} names a hub that is not valid: {source}.")]
    BadHub { name: String, source: OriginError },
}

impl Config {
    /// The file in `dir`; an empty configuration when there is none.
    ///
    /// # Errors
    /// [`ConfigError`] when it cannot be read or is not a configuration.
    pub fn load(dir: &Path) -> Result<Self, ConfigError> {
        let path = dir.join(FILE_NAME);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(read_error(&path, &error)),
        };
        let mut bytes = Vec::new();
        file.take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| read_error(&path, &error))?;
        if bytes.len() > LIMIT {
            return Err(ConfigError::Invalid { path });
        }
        serde_json::from_slice(&bytes).map_err(|_| ConfigError::Invalid { path })
    }

    /// Writes the configuration to `dir`, whole, by rename.
    ///
    /// # Errors
    /// [`ConfigError::Write`] when it cannot be written.
    ///
    /// # Panics
    /// Never: a configuration is strings in maps, which always serialise.
    pub fn save(&self, dir: &Path) -> Result<(), ConfigError> {
        let mut bytes = serde_json::to_vec_pretty(self).expect("a configuration serialises");
        bytes.push(b'\n');
        private_file::write(dir, FILE_NAME, &bytes).map_err(|source| ConfigError::Write {
            path: dir.join(FILE_NAME),
            source,
        })
    }

    /// The hub of context `name`, or `None` when there is no such context.
    ///
    /// # Errors
    /// [`ConfigError::BadHub`] when its hub text is not an origin (the file was edited by hand).
    pub fn hub_of(&self, name: &str) -> Result<Option<Origin>, ConfigError> {
        let Some(context) = self.contexts.get(name) else {
            return Ok(None);
        };
        Origin::parse(&context.hub)
            .map(Some)
            .map_err(|source| ConfigError::BadHub {
                name: name.to_owned(),
                source,
            })
    }
}

fn read_error(path: &Path, error: &io::Error) -> ConfigError {
    ConfigError::Read {
        path: path.to_owned(),
        reason: error.to_string(),
    }
}
