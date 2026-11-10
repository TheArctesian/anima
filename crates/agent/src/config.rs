//! The agent's own configuration file: which hub it speaks to and for which device
//! (MET-002 lets the agent write its own configuration).

pub mod home;
pub mod origin;
pub mod private_file;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use origin::{Origin, OriginError};

/// What `enrol` stores and `run` reads. The token is not here (MET-013).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub hub: Origin,
    pub device: Uuid,
    pub device_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    hub: String,
    device: Uuid,
    device_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("the configuration file is not valid JSON of the agent's form")]
    Unreadable,
    #[error("the configuration file's hub: {0}")]
    Hub(#[from] OriginError),
}

impl Config {
    /// Reads a configuration file. The stored hub is held to the https rule again.
    ///
    /// # Errors
    /// [`ConfigError`] when the bytes are not a configuration or name a non-https hub.
    pub fn parse(bytes: &[u8]) -> Result<Self, ConfigError> {
        let stored: Stored = serde_json::from_slice(bytes).map_err(|_| ConfigError::Unreadable)?;
        Ok(Self {
            hub: Origin::parse(&stored.hub)?,
            device: stored.device,
            device_name: stored.device_name,
        })
    }

    /// The file's bytes.
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "a struct of strings and a UUID always serialises"
    )]
    pub fn render(&self) -> Vec<u8> {
        let stored = Stored {
            hub: self.hub.as_text(),
            device: self.device,
            device_name: self.device_name.clone(),
        };
        serde_json::to_vec_pretty(&stored).expect("a configuration serialises")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            hub: Origin::default_hub(),
            device: Uuid::from_u128(7),
            device_name: "pop-os".to_owned(),
        }
    }

    #[test]
    fn met_002_a_configuration_reads_back_as_written() {
        assert_eq!(Config::parse(&config().render()).expect("reads"), config());
    }

    #[test]
    fn met_020_a_file_that_names_an_http_hub_is_refused() {
        let text = br#"{"hub":"http://hub.anima.ing","device":"00000000-0000-0000-0000-000000000007","deviceName":"x"}"#;
        assert!(matches!(
            Config::parse(text),
            Err(ConfigError::Hub(OriginError::NotHttps))
        ));
        assert!(matches!(Config::parse(b"{}"), Err(ConfigError::Unreadable)));
    }
}
