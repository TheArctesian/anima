//! Database records of the `checks` schema, and how its constraints map to field errors.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::{
    checks::{
        Error,
        config::{CheckConfig, CheckType},
        evaluation::{CheckSettings, CheckState},
        model::{Check, Due, FieldError, PushToken},
        status::Standing,
    },
    ids::{CheckId, DeviceId},
    status::Status,
};

#[derive(sqlx::FromRow)]
pub(super) struct CheckRow {
    id: CheckId,
    device_id: DeviceId,
    parent_id: Option<CheckId>,
    #[sqlx(rename = "type")]
    kind: String,
    name: String,
    interval_seconds: i32,
    retries: i32,
    retry_interval_seconds: i32,
    degraded_threshold_ms: i32,
    invert: bool,
    paused: bool,
    config: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    status: String,
    failures: i32,
    next_due_at: Option<DateTime<Utc>>,
    last_beat_at: Option<DateTime<Utc>>,
    cert_not_after: Option<DateTime<Utc>>,
    resolved_ip: Option<IpAddr>,
    push_token: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(super) struct StandingRow {
    id: CheckId,
    device_id: DeviceId,
    parent_id: Option<CheckId>,
    #[sqlx(rename = "type")]
    kind: String,
    paused: bool,
    status: String,
}

/// A stored value outside the domain of its column.
#[derive(Debug, thiserror::Error)]
#[error("a stored check value is out of range")]
struct OutOfRange;

fn out_of_range() -> sqlx::Error {
    sqlx::Error::Decode(Box::new(OutOfRange))
}

pub(super) const fn status_sql(status: Status) -> &'static str {
    match status {
        Status::Up => "up",
        Status::Down => "down",
        Status::Degraded => "degraded",
        Status::Maintenance => "maintenance",
        Status::Unknown => "unknown",
    }
}

pub(super) fn status_of(text: &str) -> Result<Status, sqlx::Error> {
    Ok(match text {
        "up" => Status::Up,
        "down" => Status::Down,
        "degraded" => Status::Degraded,
        "maintenance" => Status::Maintenance,
        "unknown" => Status::Unknown,
        _ => return Err(out_of_range()),
    })
}

pub(super) fn number<T: TryFrom<i32>>(value: i32) -> Result<T, sqlx::Error> {
    T::try_from(value).map_err(|_| out_of_range())
}

/// The `config` column: the JSON of [`CheckConfig`] without its type tag, which the `type`
/// column holds.
pub(super) fn stored_config(config: &CheckConfig) -> Result<Value, sqlx::Error> {
    let mut value =
        serde_json::to_value(config).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
    if let Value::Object(fields) = &mut value {
        fields.remove("type");
    }
    Ok(value)
}

/// Reads the `config` column back with the type tag from the `type` column. A stored tag,
/// or any key on a type without settings (serde ignores those), is refused like any other
/// unknown key.
pub(super) fn config_of(kind: &str, mut value: Value) -> Option<CheckConfig> {
    let Value::Object(fields) = &mut value else {
        return None;
    };
    let settingless = [CheckType::Push.as_sql(), CheckType::Group.as_sql()].contains(&kind);
    if fields.contains_key("type") || (settingless && !fields.is_empty()) {
        return None;
    }
    fields.insert("type".to_owned(), Value::String(kind.to_owned()));
    serde_json::from_value(value).ok()
}

impl CheckRow {
    fn settings(&self) -> Result<CheckSettings, sqlx::Error> {
        Ok(CheckSettings {
            interval_seconds: number(self.interval_seconds)?,
            retries: number(self.retries)?,
            retry_interval_seconds: number(self.retry_interval_seconds)?,
            degraded_threshold_ms: number(self.degraded_threshold_ms)?,
            invert: self.invert,
        })
    }

    fn state(&self) -> Result<CheckState, sqlx::Error> {
        Ok(CheckState {
            status: status_of(&self.status)?,
            failures: number(self.failures)?,
            next_due_at: self.next_due_at,
        })
    }

    pub(super) fn into_due(self) -> Result<Due, Error> {
        Ok(Due {
            settings: self.settings()?,
            state: self.state()?,
            id: self.id,
            device: self.device_id,
            parent: self.parent_id,
            config: config_of(&self.kind, self.config),
        })
    }
}

impl TryFrom<CheckRow> for Check {
    type Error = Error;

    fn try_from(row: CheckRow) -> Result<Self, Error> {
        let settings = row.settings()?;
        let state = row.state()?;
        let config = config_of(&row.kind, row.config).ok_or(Error::Unreadable(row.id))?;
        Ok(Self {
            id: row.id,
            device: row.device_id,
            parent: row.parent_id,
            name: row.name,
            config,
            settings,
            paused: row.paused,
            state,
            last_beat_at: row.last_beat_at,
            cert_not_after: row.cert_not_after,
            resolved_ip: row.resolved_ip,
            push_token: row.push_token.map(PushToken::stored),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub(super) fn to_checks(rows: Vec<CheckRow>) -> Result<Vec<Check>, Error> {
    rows.into_iter().map(Check::try_from).collect()
}

impl TryFrom<StandingRow> for Standing {
    type Error = Error;

    fn try_from(row: StandingRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            device: row.device_id,
            parent: row.parent_id,
            group: row.kind == CheckType::Group.as_sql(),
            paused: row.paused,
            status: status_of(&row.status)?,
        })
    }
}

/// Turns a violated constraint into the field the operator must fix.
pub(super) fn write_error(error: sqlx::Error) -> Error {
    let (field, message) = match error.as_database_error().and_then(|e| e.constraint()) {
        Some("checks_device_id_name_key") => {
            ("name", "Another check on this device has that name.")
        }
        Some("checks_name_check") => ("name", "A check name is 1 to 200 characters."),
        Some("checks_interval_seconds_check") => {
            ("intervalSeconds", "The interval is 10 to 86400 seconds.")
        }
        Some("checks_retries_check") => ("retries", "Retries are 0 to 10."),
        Some("checks_retry_interval_seconds_check") => (
            "retryIntervalSeconds",
            "The retry interval is 5 to 3600 seconds.",
        ),
        Some("checks_degraded_threshold_ms_check") => (
            "degradedThresholdMs",
            "The Degraded limit is 1 to 600000 milliseconds.",
        ),
        Some("checks_parent_fkey") => (
            "parentId",
            "The parent must be a group check on the same device.",
        ),
        Some("checks_group_parent_check") => ("parentId", "A group cannot belong to a group."),
        Some("checks_device_id_fkey") => ("deviceId", "There is no such device."),
        // The database detail of this error holds the token, so the error is dropped.
        Some("checks_push_token_key") => return Error::PushTokenTaken,
        _ => return Error::Database(error),
    };
    Error::Validation(vec![FieldError::new(field, message)])
}
