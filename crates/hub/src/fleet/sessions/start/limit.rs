//! The device's session limit (RA-036, K4 §1 step 1): a start is refused while the device has
//! as many live sessions as its limit allows. Only `start` writes session rows, so files
//! tunnels, which hold none, never count.

use sqlx::PgConnection;

use crate::{
    fleet::{
        devices,
        sessions::{Conflict, Error, device_error, store},
    },
    ids::DeviceId,
};

/// Refuses a start on `device` when it has reached its session limit; a device with no limit
/// never refuses. On its own this only answers early: two starts can both pass it.
///
/// # Errors
/// `Conflict(SessionLimit)` as above; `NotFound` when the device is gone; `Devices` or
/// `Database` when a read fails.
pub(super) async fn check(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    let limit = devices::session_limit(&mut *conn, device)
        .await
        .map_err(|error| device_error(device, error))?;
    let Some(limit) = limit else {
        return Ok(());
    };
    let live = store::live_count(conn, device).await?;
    if live >= i64::from(limit.get()) {
        return Err(Error::Conflict(Conflict::SessionLimit));
    }
    Ok(())
}

/// [`check`] under the device's start lock, which `conn`'s transaction then holds until it
/// commits or rolls back, so no other start on the device counts before this one's row is
/// visible. The count reads after the lock is taken: under read committed each statement sees
/// what committed before it began, so the row of the start that held the lock is counted.
///
/// # Errors
/// As [`check`].
pub(super) async fn check_locked(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    store::lock_starts(&mut *conn, device).await?;
    check(conn, device).await
}
