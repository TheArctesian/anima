//! SQL for the remote access settings and the wake relay: `rdp_settings`, `vnc_settings`, the
//! remote columns of `devices` and `sites.wake_relay_device_id`. A device row always has its
//! `rdp_settings` and `vnc_settings` rows (a trigger adds them), so a write only updates.
//!
//! A relay change locks the device row before the site row, the order `update_device` and
//! `delete_device` take them in, so the two cannot deadlock.

use sqlx::{PgConnection, PgPool};

use super::fetch;
use crate::{
    devices::{
        Device, Error, Missing,
        model::Site,
        records::{DeviceRow, to_devices, to_site},
        remote::{
            MetricsSource, RdpSettings, RemoteAccess, RemoteChange, TransferPolicy, VncSettings,
        },
    },
    ids::{DeviceId, SiteId},
};

/// Writes the remote access part of a device just inserted.
pub(in crate::devices) async fn write_new(
    conn: &mut PgConnection,
    id: DeviceId,
    remote: &RemoteAccess,
) -> Result<(), Error> {
    write_rdp(conn, id, &remote.rdp).await?;
    write_vnc(conn, id, &remote.vnc).await?;
    sqlx::query(
        "UPDATE devices.devices SET mac_address = $2::text::macaddr, metrics_source = $3, \
                session_limit = $4, record_sessions = $5 WHERE id = $1",
    )
    .bind(id)
    .bind(remote.mac_address.map(|mac| mac.to_string()))
    .bind(remote.metrics_source.as_str())
    .bind(remote.session_limit.map(|limit| i32::from(limit.get())))
    .bind(remote.record_sessions)
    .execute(&mut *conn)
    .await?;
    write_transfer(conn, id, remote.transfer).await
}

/// Writes the parts of the update that are present; each settings object is replaced wholly.
pub(in crate::devices) async fn write_change(
    conn: &mut PgConnection,
    id: DeviceId,
    change: &RemoteChange,
) -> Result<(), Error> {
    if let Some(rdp) = &change.rdp {
        write_rdp(conn, id, rdp).await?;
    }
    if let Some(vnc) = &change.vnc {
        write_vnc(conn, id, vnc).await?;
    }
    if let Some(transfer) = change.transfer {
        write_transfer(conn, id, transfer).await?;
    }
    sqlx::query(
        "UPDATE devices.devices SET \
            mac_address = CASE WHEN $2 THEN $3::text::macaddr ELSE mac_address END, \
            metrics_source = COALESCE($4, metrics_source), \
            session_limit = CASE WHEN $5 THEN $6 ELSE session_limit END, \
            record_sessions = COALESCE($7, record_sessions) WHERE id = $1",
    )
    .bind(id)
    .bind(change.mac_address.is_some())
    .bind(change.mac_address.flatten().map(|mac| mac.to_string()))
    .bind(change.metrics_source.map(MetricsSource::as_str))
    .bind(change.session_limit.is_some())
    .bind(
        change
            .session_limit
            .flatten()
            .map(|limit| i32::from(limit.get())),
    )
    .bind(change.record_sessions)
    .execute(conn)
    .await?;
    Ok(())
}

/// Sets the metrics source to `agent` and returns the device's name; the update locks the
/// device row first, the order `delete_device` takes it in.
pub(in crate::devices) async fn set_agent_source(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<String, Error> {
    let name: Option<String> = sqlx::query_scalar(
        "UPDATE devices.devices SET metrics_source = $2 WHERE id = $1 RETURNING name",
    )
    .bind(id)
    .bind(MetricsSource::Agent.as_str())
    .fetch_optional(conn)
    .await?;
    name.ok_or(Error::NotFound(Missing::Device(id)))
}

async fn write_rdp(conn: &mut PgConnection, id: DeviceId, rdp: &RdpSettings) -> Result<(), Error> {
    sqlx::query(
        "UPDATE devices.rdp_settings SET enabled = $2, port = $3, username = $4, domain = $5, \
                security = $6, ignore_certificate = $7, resize_method = $8, audio_out = $9, \
                microphone = $10, shared_drive = $11 WHERE device_id = $1",
    )
    .bind(id)
    .bind(rdp.enabled)
    .bind(i32::from(rdp.port))
    .bind(&rdp.username)
    .bind(rdp.domain.as_deref())
    .bind(rdp.security.as_str())
    .bind(rdp.ignore_certificate)
    .bind(rdp.resize.as_str())
    .bind(rdp.audio_out)
    .bind(rdp.microphone)
    .bind(rdp.shared_drive)
    .execute(conn)
    .await?;
    Ok(())
}

async fn write_vnc(conn: &mut PgConnection, id: DeviceId, vnc: &VncSettings) -> Result<(), Error> {
    sqlx::query(
        "UPDATE devices.vnc_settings SET enabled = $2, port = $3, username = $4 WHERE device_id = $1",
    )
    .bind(id)
    .bind(vnc.enabled)
    .bind(i32::from(vnc.port))
    .bind(vnc.username.as_deref())
    .execute(conn)
    .await?;
    Ok(())
}

async fn write_transfer(
    conn: &mut PgConnection,
    id: DeviceId,
    transfer: TransferPolicy,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE devices.devices SET transfer_upload = $2, transfer_download = $3, \
                transfer_copy = $4, transfer_paste = $5 WHERE id = $1",
    )
    .bind(id)
    .bind(transfer.upload)
    .bind(transfer.download)
    .bind(transfer.copy)
    .bind(transfer.paste)
    .execute(conn)
    .await?;
    Ok(())
}

/// Clears every relay role the device can no longer fill: at a site other than `site` (when
/// the device moved) and at any site when its SSH was turned off (the relay works over SSH).
pub(in crate::devices) async fn release_relay(
    conn: &mut PgConnection,
    device: DeviceId,
    site: Option<SiteId>,
    ssh_off: bool,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE devices.sites SET wake_relay_device_id = NULL \
         WHERE wake_relay_device_id = $1 AND ($3 OR id <> COALESCE($2, id))",
    )
    .bind(device)
    .bind(site)
    .bind(ssh_off)
    .execute(conn)
    .await?;
    Ok(())
}

/// The site's wake relay device, if one is set.
pub(in crate::devices) async fn wake_relay(
    db: &PgPool,
    site: SiteId,
) -> Result<Option<Device>, Error> {
    let relay: Option<Option<DeviceId>> =
        sqlx::query_scalar("SELECT wake_relay_device_id FROM devices.sites WHERE id = $1")
            .bind(site)
            .fetch_optional(db)
            .await?;
    match relay.ok_or(Error::NotFound(Missing::Site(site)))? {
        None => Ok(None),
        Some(device) => match fetch(db, device).await {
            Ok(device) => Ok(Some(device)),
            // Deleted between the two reads: the foreign key has cleared the relay.
            Err(Error::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        },
    }
}

/// The devices with these ids, for `Site.wakeRelay`; unknown ids are left out.
pub(in crate::devices) async fn devices_by_ids(
    db: &PgPool,
    ids: &[DeviceId],
) -> Result<Vec<Device>, Error> {
    let rows = sqlx::query_as(select_device!("WHERE d.id = ANY($1)"))
        .bind(ids)
        .fetch_all(db)
        .await?;
    to_devices(rows)
}

/// The device, with its row and its SSH settings row share-locked until the transaction ends,
/// so it cannot move to another site or turn SSH off before the relay is written, and a
/// concurrent change is read as committed; `None` when there is none.
pub(in crate::devices) async fn share_device(
    conn: &mut PgConnection,
    id: DeviceId,
) -> Result<Option<Device>, Error> {
    let row = sqlx::query_as::<_, DeviceRow>(select_device!("WHERE d.id = $1 FOR SHARE OF d, s"))
        .bind(id)
        .fetch_optional(conn)
        .await?;
    Ok(row.map(Device::try_from).transpose()?)
}

/// The site, with its row locked until the transaction ends.
pub(in crate::devices) async fn lock_site(
    conn: &mut PgConnection,
    site: SiteId,
) -> Result<Site, Error> {
    sqlx::query_as(
        "SELECT id, name, subnet, wake_relay_device_id FROM devices.sites WHERE id = $1 FOR UPDATE",
    )
    .bind(site)
    .fetch_optional(conn)
    .await?
    .map(to_site)
    .ok_or(Error::NotFound(Missing::Site(site)))
}

/// Sets or, with `None`, clears the site's wake relay.
pub(in crate::devices) async fn set_wake_relay(
    conn: &mut PgConnection,
    site: SiteId,
    device: Option<DeviceId>,
) -> Result<Site, Error> {
    sqlx::query_as(
        "UPDATE devices.sites SET wake_relay_device_id = $2 WHERE id = $1 \
         RETURNING id, name, subnet, wake_relay_device_id",
    )
    .bind(site)
    .bind(device)
    .fetch_optional(conn)
    .await?
    .map(to_site)
    .ok_or(Error::NotFound(Missing::Site(site)))
}
