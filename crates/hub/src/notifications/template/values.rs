//! The values of one notification as a template shows them: read from the outbox payload that
//! `observe` writes, and cleaned again with `text::clean` (Q2-11, D-21).

use serde_json::Value;

use super::Placeholder;
use crate::text::clean;

const NAME_CHARS: usize = 200;
const MESSAGE_CHARS: usize = 500;
const STATUS_CHARS: usize = 20;
const AT_CHARS: usize = 40;

/// What one notification says. A value the payload lacks is `None` and renders as an empty
/// string; `days` is set for the two expiry kinds only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::notifications) struct Values {
    pub(in crate::notifications) device: Option<String>,
    pub(in crate::notifications) check: Option<String>,
    pub(in crate::notifications) status: Option<String>,
    pub(in crate::notifications) previous: Option<String>,
    pub(in crate::notifications) message: Option<String>,
    pub(in crate::notifications) at: Option<String>,
    pub(in crate::notifications) days: Option<u32>,
}

impl Values {
    /// The values of an outbox payload (keys `device`, `check`, `status`, `previous`,
    /// `message`, `at`, `days`). Every text is cleaned and cut again, whatever wrote the row;
    /// a key of another type is a missing value.
    #[must_use]
    pub(in crate::notifications) fn from_payload(payload: &Value) -> Self {
        let text = |key: &str, max: usize| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .map(|value| clean(value, max))
        };
        Self {
            device: text("device", NAME_CHARS),
            check: text("check", NAME_CHARS),
            status: text("status", STATUS_CHARS),
            previous: text("previous", STATUS_CHARS),
            message: text("message", MESSAGE_CHARS),
            at: text("at", AT_CHARS),
            days: payload
                .get("days")
                .and_then(Value::as_u64)
                .and_then(|days| u32::try_from(days).ok()),
        }
    }

    pub(super) fn get(&self, placeholder: Placeholder) -> String {
        let text = match placeholder {
            Placeholder::Device => &self.device,
            Placeholder::Check => &self.check,
            Placeholder::Status => &self.status,
            Placeholder::Previous => &self.previous,
            Placeholder::Message => &self.message,
            Placeholder::At => &self.at,
            Placeholder::Days => return self.days.map(|days| days.to_string()).unwrap_or_default(),
        };
        text.clone().unwrap_or_default()
    }
}
