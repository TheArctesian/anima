//! The installation's notification settings: one row (D-21, D-22).

use sqlx::{PgConnection, PgExecutor};

use super::rows::SettingsRow;
use crate::notifications::{Error, model::InstallationSettings};

/// The settings. The migration seeds the row, so it always exists.
pub(in crate::notifications) async fn installation_settings(
    db: impl PgExecutor<'_>,
) -> Result<InstallationSettings, Error> {
    let row: SettingsRow = sqlx::query_as(
        "SELECT certificate_days, title_template, body_template FROM notifications.settings",
    )
    .fetch_one(db)
    .await?;
    InstallationSettings::try_from(row)
}

/// Replaces the settings.
pub(in crate::notifications) async fn write_installation_settings(
    conn: &mut PgConnection,
    settings: &InstallationSettings,
) -> Result<(), Error> {
    let days: Vec<i32> = settings
        .certificate_days
        .iter()
        .map(|d| i32::from(*d))
        .collect();
    sqlx::query(
        "UPDATE notifications.settings SET certificate_days = $1, title_template = $2, \
         body_template = $3",
    )
    .bind(days)
    .bind(&settings.title_template)
    .bind(&settings.body_template)
    .execute(conn)
    .await?;
    Ok(())
}
