//! The non-secret settings of a channel, typed per kind and stored as JSON in
//! `channels.settings` (K2). The API shows them as they are (SEC-005).

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{AppriseSettings, EmailSettings, NtfySettings, WebhookSettings};
use crate::fleet::notifications::{ChannelKind, Error};

/// The HTTP method of a webhook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(in crate::fleet::notifications) enum WebhookMethod {
    Post,
    Put,
}

/// How an Apprise channel names its targets: URLs sent with each notification, or the key of
/// a configuration stored in the Apprise API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::fleet::notifications) enum AppriseMode {
    StatelessUrls,
    ConfigKey,
}

/// The settings of one channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::notifications) enum Settings {
    Webhook(WebhookSettings),
    Ntfy(NtfySettings),
    Email(EmailSettings),
    Apprise(AppriseSettings),
}

/// The settings of `kind` in `value`; every field must be there and no other.
fn read<T: DeserializeOwned>(value: &serde_json::Value) -> Result<T, Error> {
    T::deserialize(value).map_err(|_| Error::Unreadable)
}

impl Settings {
    pub(in crate::fleet::notifications) const fn kind(&self) -> ChannelKind {
        match self {
            Self::Webhook(_) => ChannelKind::Webhook,
            Self::Ntfy(_) => ChannelKind::Ntfy,
            Self::Email(_) => ChannelKind::Email,
            Self::Apprise(_) => ChannelKind::Apprise,
        }
    }

    /// The settings stored for a channel of `kind`.
    ///
    /// # Errors
    /// `Unreadable` when the JSON is not the settings of that kind.
    pub(in crate::fleet::notifications) fn from_stored(
        kind: ChannelKind,
        value: &serde_json::Value,
    ) -> Result<Self, Error> {
        Ok(match kind {
            ChannelKind::Webhook => Self::Webhook(read(value)?),
            ChannelKind::Ntfy => Self::Ntfy(read(value)?),
            ChannelKind::Email => Self::Email(read(value)?),
            ChannelKind::Apprise => Self::Apprise(read(value)?),
        })
    }

    /// The JSON stored in `channels.settings`.
    pub(in crate::fleet::notifications) fn to_stored(&self) -> serde_json::Value {
        let stored = match self {
            Self::Webhook(settings) => serde_json::to_value(settings),
            Self::Ntfy(settings) => serde_json::to_value(settings),
            Self::Email(settings) => serde_json::to_value(settings),
            Self::Apprise(settings) => serde_json::to_value(settings),
        };
        stored.expect("settings of strings, numbers, bools and enum tags always serialise (CQ-023)")
    }

    /// Where the secrets go: the webhook origin, the ntfy server, the mail server's host and
    /// port, or the Apprise server. Sealed secrets are bound to it (SEC-003, CR-14 item 1).
    pub(in crate::fleet::notifications) fn target(&self) -> String {
        match self {
            Self::Webhook(settings) => settings.origin.clone(),
            Self::Ntfy(settings) => settings.server.clone(),
            Self::Email(settings) => format!("{}:{}", settings.host, settings.port),
            Self::Apprise(settings) => settings.server.clone(),
        }
    }

    /// The names the secrets are sent under: the webhook's header names in lower case, in
    /// order. Sealed secrets are bound to them too.
    pub(in crate::fleet::notifications) fn secret_names(&self) -> Vec<String> {
        match self {
            Self::Webhook(settings) => settings
                .header_names
                .iter()
                .map(|name| name.to_ascii_lowercase())
                .collect(),
            Self::Ntfy(_) | Self::Email(_) | Self::Apprise(_) => Vec::new(),
        }
    }
}
