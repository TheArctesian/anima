//! The K1 types of web interfaces: `WebInterface`, `WebInterfaceSuggestion`, `OpenIn` and
//! `WebInterfaceInput`. The domain types are the output objects, so `Device.webInterfaces`
//! returns them as they are.

use std::str::FromStr;

use async_graphql::{ID, InputObject, Object, Result};
use chrono::{DateTime, Utc};

use super::open_in::OpenInValue;
use crate::api_errors::invalid;
use crate::web_interfaces::{FieldError, Suggestion, WebInterface, WebInterfaceInput};

#[Object(name = "WebInterface")]
impl WebInterface {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn device_id(&self) -> ID {
        ID(self.device.to_string())
    }

    async fn label(&self) -> &str {
        &self.label
    }

    async fn url(&self) -> &str {
        &self.url
    }

    async fn open_in(&self) -> OpenInValue {
        self.open_in.into()
    }

    /// Lower-case hex, 64 digits.
    async fn pinned_fingerprint_sha256(&self) -> Option<&str> {
        self.pin.as_ref().map(|pin| pin.fingerprint.as_str())
    }

    async fn pinned_at(&self) -> Option<DateTime<Utc>> {
        self.pin.as_ref().map(|pin| pin.at)
    }
}

#[Object(name = "WebInterfaceSuggestion")]
impl Suggestion {
    async fn check_id(&self) -> ID {
        ID(self.check.to_string())
    }

    async fn url(&self) -> &str {
        &self.url
    }

    async fn label(&self) -> &str {
        &self.label
    }
}

#[derive(InputObject)]
#[graphql(name = "WebInterfaceInput")]
pub(super) struct WebInterfaceFields {
    /// Defaults to the URL's host.
    label: Option<String>,
    url: String,
    /// K1 types it nullable with a default; a null means the default.
    #[graphql(default_with = "Some(OpenInValue::AnimaTab)")]
    open_in: Option<OpenInValue>,
}

impl From<WebInterfaceFields> for WebInterfaceInput {
    fn from(fields: WebInterfaceFields) -> Self {
        Self {
            label: fields.label,
            url: fields.url,
            open_in: fields.open_in.unwrap_or(OpenInValue::AnimaTab).into(),
        }
    }
}

/// The typed id an `ID` argument names; the error is `VALIDATION` on `field`.
pub(super) fn parse_id<T: FromStr>(id: &ID, field: &'static str) -> Result<T> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new(field, "That id is not valid.")]))
}
