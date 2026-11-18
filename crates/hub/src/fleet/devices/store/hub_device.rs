//! SQL for the hub's own device (HMD-045): the lock of its start, the device at a host, the
//! names in use and the site of the link.

use ipnet::IpNet;
use sqlx::PgConnection;

use crate::fleet::devices::{
    Error,
    model::{Device, Site},
    records::{DeviceRow, to_site},
};

/// Waits for, then holds until the caller's transaction ends, the lock of the hub's own
/// device, so two hubs that start on one database add one device.
pub(in crate::fleet::devices) async fn lock_hub_device(
    conn: &mut PgConnection,
) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('devices:hub', 0))")
        .execute(conn)
        .await?;
    Ok(())
}

/// The oldest device whose host is `host` in any letter case.
pub(in crate::fleet::devices) async fn device_at_host(
    conn: &mut PgConnection,
    host: &str,
) -> Result<Option<Device>, Error> {
    let row = sqlx::query_as::<_, DeviceRow>(select_device!(
        "WHERE lower(d.host) = lower($1) ORDER BY d.created_at, d.id LIMIT 1"
    ))
    .bind(host)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(Device::try_from).transpose()?)
}

/// Those of `names` that a device has.
pub(in crate::fleet::devices) async fn names_in_use(
    conn: &mut PgConnection,
    names: &[String],
) -> Result<Vec<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT name FROM devices.devices WHERE name = ANY($1)")
            .bind(names)
            .fetch_all(conn)
            .await?,
    )
}

/// The site with this name, else a site whose subnet overlaps `subnet`: a new site with that
/// subnet could not be stored beside it.
pub(in crate::fleet::devices) async fn site_for(
    conn: &mut PgConnection,
    name: &str,
    subnet: IpNet,
) -> Result<Option<Site>, Error> {
    let row = sqlx::query_as(
        "SELECT id, name, subnet, wake_relay_device_id FROM devices.sites \
         WHERE name = $1 OR subnet && $2 ORDER BY (name = $1) DESC, name COLLATE \"C\" LIMIT 1",
    )
    .bind(name)
    .bind(subnet)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(to_site))
}
