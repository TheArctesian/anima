//! The common settings of a check input (interval, retries, retry interval, Degraded limit),
//! checked for range. Pure; errors name the field, never its value.

use super::form::{keep, number};
use crate::fleet::checks::FieldError;

/// The common settings an operator sent, in range or reported.
pub(super) struct Settings {
    pub(super) interval_seconds: Option<u32>,
    pub(super) retries: Option<u8>,
    pub(super) retry_interval_seconds: Option<u32>,
    pub(super) degraded_threshold_ms: Option<u32>,
}

pub(super) struct RawSettings {
    pub(super) interval_seconds: Option<i32>,
    pub(super) retries: Option<i32>,
    pub(super) retry_interval_seconds: Option<i32>,
    pub(super) degraded_threshold_ms: Option<i32>,
}

fn optional<T: TryFrom<i32>>(
    field: &'static str,
    value: Option<i32>,
    errors: &mut Vec<FieldError>,
) -> Option<T> {
    value.and_then(|value| keep(number(field, value), errors))
}

impl RawSettings {
    pub(super) fn convert(self, errors: &mut Vec<FieldError>) -> Settings {
        Settings {
            interval_seconds: optional("intervalSeconds", self.interval_seconds, errors),
            retries: optional("retries", self.retries, errors),
            retry_interval_seconds: optional(
                "retryIntervalSeconds",
                self.retry_interval_seconds,
                errors,
            ),
            degraded_threshold_ms: optional(
                "degradedThresholdMs",
                self.degraded_threshold_ms,
                errors,
            ),
        }
    }
}
