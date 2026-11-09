//! SQL that holds a device row for another domain's transaction until it ends, so that
//! transaction takes the device row before its own rows, the order `delete_device` takes them in.

use sqlx::PgConnection;

use crate::{
    devices::{Error, Missing},
    ids::DeviceId,
};

/// Locks the device row `FOR UPDATE` until the transaction ends.
pub(in crate::devices) async fn lock_device(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<(), Error> {
    let row: Option<DeviceId> =
        sqlx::query_scalar("SELECT id FROM devices.devices WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    row.map(|_| ()).ok_or(Error::NotFound(Missing::Device(id)))
}
