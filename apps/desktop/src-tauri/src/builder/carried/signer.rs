//! Who signs a macOS release (BLD-083): the Developer ID identity this builder was built with,
//! when the clone's committed `tauri.conf.json` names the same one, and the Team ID it names.
//! The decision is pure; reading the clone's file is the edge.

use std::{io::Read, path::Path};

use super::mach_o::TeamId;
use crate::builder::{artifact, error::BuilderError};

/// The Tauri configuration that names the signing identity, relative to the clone.
pub const CONFIG: &str = "apps/desktop/src-tauri/tauri.conf.json";
/// The most of `tauri.conf.json` that is read.
const CONFIG_BYTES_MAX: u64 = 1 << 20;

/// The identity that signs a release and the Team ID it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub identity: String,
    pub team: TeamId,
}

impl Signer {
    /// The signer of a release: `own`, the identity this builder was built with, when the clone's
    /// `tauri.conf.json` names the same one. The identity never comes from the clone, which a
    /// build can change.
    pub fn pinned(own: Option<&str>, clone: &str) -> Result<Self, BuilderError> {
        match own {
            Some(own) if !own.is_empty() && own == clone => Self::from_identity(own),
            _ => Err(BuilderError::Bundle),
        }
    }

    /// `identity` and the Team ID in its last parentheses ([`TeamId::of_identity`]).
    pub fn from_identity(identity: &str) -> Result<Self, BuilderError> {
        Ok(Self {
            identity: identity.to_owned(),
            team: TeamId::of_identity(identity)?,
        })
    }
}

/// `bundle.macOS.signingIdentity` of a `tauri.conf.json`, when it names one.
pub fn configured(config: &serde_json::Value) -> Option<&str> {
    config["bundle"]["macOS"]["signingIdentity"]
        .as_str()
        .filter(|name| !name.is_empty())
}

/// The identity the clone's own `tauri.conf.json` names; `Bundle` when the file is not the
/// build's own or names none.
pub fn in_clone(clone: &Path) -> Result<String, BuilderError> {
    let file = artifact::open_own(clone, &clone.join(CONFIG))?;
    let config: serde_json::Value =
        serde_json::from_reader(file.take(CONFIG_BYTES_MAX)).map_err(|_| BuilderError::Bundle)?;
    configured(&config)
        .map(str::to_owned)
        .ok_or(BuilderError::Bundle)
}
