//! Maintenance windows: when a device or check is expected to be down on purpose (spec 6.5).

mod active;
pub mod graphql;
mod model;
mod records;
mod rules;
mod schedule;
mod store;
mod targets;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::api_errors::field_names;
use crate::ids::{CheckId, DeviceId, WindowId};

pub use active::ActiveTargets;
pub use model::{Target, TargetType, Weekday, Window, WindowKind, WindowRule};
pub use rules::FieldError;
pub use schedule::is_active;

/// Why a maintenance operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("maintenance window {0} not found")]
    NotFound(WindowId),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// What the windows in force at `now` apply to, from one query. Ask it about every device in
/// a list instead of calling `active_for` once for each (API-005).
///
/// # Errors
/// `Database` when the query fails.
pub async fn active_targets(db: &PgPool, now: DateTime<Utc>) -> Result<ActiveTargets, Error> {
    let windows = store::all(db).await?;
    Ok(ActiveTargets::of(&windows, now))
}

/// Whether an active window applies to the device, to the check, or to any of the tags. The
/// caller passes the device's tags; maintenance does not read devices.
///
/// # Errors
/// `Database` when the query fails.
#[expect(
    clippy::too_many_arguments,
    reason = "the hub contract fixes the signature of active_for"
)]
pub async fn active_for(
    db: &PgPool,
    device: DeviceId,
    check: Option<CheckId>,
    tags: &[String],
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    Ok(active_targets(db, now).await?.covers(device, check, tags))
}
