//! SQL that holds a device row for another domain's transaction until it ends, so that
//! transaction takes the device row before its own rows, the order `delete_device` takes them in.

use sqlx::PgConnection;

use crate::{
    devices::{Error, Hold, Missing},
    ids::DeviceId,
};

/// Holds the device row as `hold` says until the transaction ends.
pub(in crate::devices) async fn hold_device(
    conn: &mut PgConnection,
    id: DeviceId,
    hold: Hold,
) -> Result<(), Error> {
    let sql = match hold {
        Hold::Kept => "SELECT id FROM devices.devices WHERE id = $1 FOR KEY SHARE",
        Hold::Locked => "SELECT id FROM devices.devices WHERE id = $1 FOR UPDATE",
    };
    let row: Option<DeviceId> = sqlx::query_scalar(sql)
        .bind(id)
        .fetch_optional(conn)
        .await?;
    row.map(|_| ()).ok_or(Error::NotFound(Missing::Device(id)))
}
