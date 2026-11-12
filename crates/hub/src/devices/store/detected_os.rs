//! SQL for the operating system detected on a device: the `os` column of `devices`, written
//! only while it is `other` (DEV-025). The operator's choice is never overwritten.

use sqlx::PgPool;

use crate::{
    devices::{Error, OperatingSystem},
    ids::DeviceId,
};

/// Sets the operating system detected on the device while its stored one is Other; returns
/// whether it was written.
pub(in crate::devices) async fn set_detected_os(
    db: &PgPool,
    id: DeviceId,
    os: OperatingSystem,
) -> Result<bool, Error> {
    let done = sqlx::query("UPDATE devices.devices SET os = $2 WHERE id = $1 AND os = 'other'")
        .bind(id)
        .bind(os.as_str())
        .execute(db)
        .await?;
    Ok(done.rows_affected() == 1)
}
