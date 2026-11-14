//! The attempt to connect GitHub as the command line reads it: the K1 `GithubConnectAttempt`
//! fields a line of output is made from. Every document of the flow answers with the same
//! shape, so each answer is read through its JSON (CMD-004).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::failure::Failure;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Phase {
    WaitingForBrowser,
    AtGithub,
    AppCreated,
    Connected,
    Failed,
    Cancelled,
    /// A phase a newer hub added.
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    Expired,
    ExchangeRefused,
    GithubUnreachable,
    ReplyUnusable,
    KeyRefused,
    #[serde(other)]
    Unknown,
}

/// The account of a connected attempt.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Account {
    pub label: String,
}

/// One answer about the attempt. `app_slug`, `owner_login` and `message` hold text from
/// GitHub: they are cleaned where they are printed (`Report`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub id: String,
    pub phase: Phase,
    pub page_url: String,
    pub app_slug: Option<String>,
    pub owner_login: Option<String>,
    pub install_url: Option<String>,
    pub app_settings_url: Option<String>,
    pub failure: Option<Reason>,
    pub message: Option<String>,
    pub account: Option<Account>,
}

impl Phase {
    /// The phase in the hub's words.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WaitingForBrowser => "WAITING_FOR_BROWSER",
            Self::AtGithub => "AT_GITHUB",
            Self::AppCreated => "APP_CREATED",
            Self::Connected => "CONNECTED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Whether the attempt has ended: nothing more will change.
    #[must_use]
    pub const fn ended(self) -> bool {
        matches!(self, Self::Connected | Self::Failed | Self::Cancelled)
    }
}

impl Attempt {
    /// The attempt in `answer`, a typed answer of any document of the flow, and its JSON as
    /// the hub's field names (for `--json`).
    ///
    /// # Errors
    /// [`Failure::Input`] when the answer is not the attempt the documents ask for.
    pub fn read<T: Serialize>(answer: &T) -> Result<(Self, Value), Failure> {
        let unreadable = |reason: String| Failure::Input {
            what: "the GitHub connection".to_owned(),
            reason,
        };
        let value = serde_json::to_value(answer).map_err(|error| unreadable(error.to_string()))?;
        let attempt =
            serde_json::from_value(value.clone()).map_err(|error| unreadable(error.to_string()))?;
        Ok((attempt, value))
    }
}
