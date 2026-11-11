//! SQL for the distribution read from a device: the `distribution` column of `devices`
//! (DEV-016). Only the metrics collection writes it; the operator never does.

use sqlx::PgPool;

use crate::{
    devices::{Distribution, Error},
    ids::DeviceId,
};

/// Sets the distribution read from the device; a value equal to the stored one writes nothing.
pub(in crate::devices) async fn set_distribution(
    db: &PgPool,
    id: DeviceId,
    distribution: &Distribution,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE devices.devices SET distribution = $2 \
         WHERE id = $1 AND distribution IS DISTINCT FROM $2",
    )
    .bind(id)
    .bind(distribution.as_str())
    .execute(db)
    .await?;
    Ok(())
}
