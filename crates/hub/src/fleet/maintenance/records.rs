//! Database records of the `maintenance` schema and how its constraints map to field errors.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sqlx::types::Json;

use super::{
    Error, FieldError,
    model::{Target, TargetType, Window, WindowRule},
};
use crate::ids::WindowId;

/// A window with its targets as two lists of the same length, in the same order.
#[derive(sqlx::FromRow)]
pub(super) struct WindowRow {
    id: WindowId,
    name: String,
    timezone: String,
    rule: Json<WindowRule>,
    active_manual: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    target_types: Vec<String>,
    targets: Vec<String>,
}

fn decode<E: std::error::Error + Send + Sync + 'static>(error: E) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

/// The database holds a target type this build does not know.
#[derive(Debug, thiserror::Error)]
#[error("unknown target type '{0}'")]
struct UnknownTargetType(String);

fn stored_target(target_type: &str, target: String) -> Result<Target, sqlx::Error> {
    match target_type {
        t if t == TargetType::Device.as_str() => target.parse().map(Target::Device).map_err(decode),
        t if t == TargetType::Check.as_str() => target.parse().map(Target::Check).map_err(decode),
        t if t == TargetType::Tag.as_str() => Ok(Target::Tag(target)),
        other => Err(decode(UnknownTargetType(other.to_owned()))),
    }
}

impl TryFrom<WindowRow> for Window {
    type Error = sqlx::Error;

    fn try_from(row: WindowRow) -> Result<Self, sqlx::Error> {
        let targets = row
            .target_types
            .iter()
            .zip(row.targets)
            .map(|(target_type, target)| stored_target(target_type, target))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            id: row.id,
            name: row.name,
            timezone: Tz::from_str(&row.timezone).map_err(decode)?,
            rule: row.rule.0,
            active_manual: row.active_manual,
            targets,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub(super) fn to_windows(rows: Vec<WindowRow>) -> Result<Vec<Window>, Error> {
    Ok(rows
        .into_iter()
        .map(Window::try_from)
        .collect::<Result<_, _>>()?)
}

/// Turns a violated constraint into the field the operator must fix.
pub(super) fn write_error(error: sqlx::Error) -> Error {
    let field = match error.as_database_error().and_then(|e| e.constraint()) {
        Some("windows_name_key") => {
            FieldError::new("name", "That name is already taken. Choose another.")
        }
        _ => return Error::Database(error),
    };
    Error::Validation(vec![field])
}
