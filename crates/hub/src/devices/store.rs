//! All SQL on schema `devices`: I/O only; rows are mapped in `records`. Ids and time come in as parameters.

use chrono::{DateTime, Utc};
use sqlx::{Connection, PgConnection, PgExecutor, PgPool};

use super::{
    Error, Missing,
    model::{Device, DeviceChange, DeviceFilter, NewDevice, NewSite, OperatingSystem, Site},
    page::split_page,
    records::{DeviceRow, to_devices, to_site, write_error},
};
use crate::ids::{DeviceId, SiteId};

/// A device with its SSH, RDP and VNC settings and tags in one row.
macro_rules! select_device {
    ($($tail:expr),*) => {
        concat!(
            "SELECT d.id, d.site_id, d.name, d.host, d.ip, d.kind, d.os, d.maintenance, \
                    d.created_at, d.updated_at, s.enabled, s.port, s.username, s.auth, \
                    ARRAY(SELECT t.tag FROM devices.device_tags t WHERE t.device_id = d.id \
                          ORDER BY t.tag COLLATE \"C\") AS tags, \
                    r.enabled AS rdp_enabled, r.port AS rdp_port, r.username AS rdp_username, \
                    r.domain AS rdp_domain, r.security AS rdp_security, \
                    r.ignore_certificate AS rdp_ignore_certificate, \
                    r.resize_method AS rdp_resize_method, r.audio_out AS rdp_audio_out, \
                    r.microphone AS rdp_microphone, r.shared_drive AS rdp_shared_drive, \
                    v.enabled AS vnc_enabled, v.port AS vnc_port, v.username AS vnc_username, \
                    d.mac_address::text AS mac_address, d.metrics_source, d.session_limit, d.record_sessions, \
                    d.transfer_upload, d.transfer_download, d.transfer_copy, d.transfer_paste \
             FROM devices.devices d JOIN devices.ssh_settings s ON s.device_id = d.id \
                  JOIN devices.rdp_settings r ON r.device_id = d.id \
                  JOIN devices.vnc_settings v ON v.device_id = d.id ",
            $($tail),*
        )
    };
}

/// Filter on `$1` name pattern, `$2` required tags, `$3` site.
macro_rules! device_filter {
    () => {
        "(($1::text IS NULL OR d.name ILIKE $1 ESCAPE '\\') \
          AND $2::text[] <@ ARRAY(SELECT t.tag FROM devices.device_tags t WHERE t.device_id = d.id) \
          AND ($3::uuid IS NULL OR d.site_id = $3))"
    };
}

mod holds;
pub(super) mod remote;

pub(super) use holds::{lock_device, lock_metrics_source};
pub(super) use remote::{
    devices_by_ids, lock_site, set_agent_source, set_wake_relay, share_device, wake_relay,
};

pub(super) async fn create_site(
    conn: &mut PgConnection,
    id: SiteId,
    site: &NewSite,
) -> Result<Site, Error> {
    sqlx::query_as("INSERT INTO devices.sites (id, name, subnet) VALUES ($1, $2, $3) RETURNING id, name, subnet, wake_relay_device_id")
        .bind(id)
        .bind(&site.name)
        .bind(site.subnet.net())
        .fetch_one(conn)
        .await
        .map(to_site)
        .map_err(write_error)
}

pub(super) async fn list_sites(db: &PgPool) -> Result<Vec<Site>, Error> {
    let rows =
        sqlx::query_as("SELECT id, name, subnet, wake_relay_device_id FROM devices.sites ORDER BY name COLLATE \"C\"")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(to_site).collect())
}

pub(super) async fn sites_by_ids(db: &PgPool, ids: &[SiteId]) -> Result<Vec<Site>, Error> {
    let rows = sqlx::query_as(
        "SELECT id, name, subnet, wake_relay_device_id FROM devices.sites WHERE id = ANY($1) ORDER BY name COLLATE \"C\"",
    )
    .bind(ids)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(to_site).collect())
}

/// Inserts the device, its tags and its settings; all or nothing.
/// An AFTER INSERT trigger adds the default `rdp_settings` and `vnc_settings` rows.
pub(super) async fn create_device(
    conn: &mut PgConnection,
    id: DeviceId,
    now: DateTime<Utc>,
    device: &NewDevice,
) -> Result<Device, Error> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, ip, kind, os, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $8, $5, $6, $7, $7)",
    )
    .bind(id)
    .bind(device.site)
    .bind(&device.name)
    .bind(&device.host)
    .bind(&device.kind)
    .bind(device.os.as_str())
    .bind(now)
    .bind(device.ip())
    .execute(&mut *tx)
    .await
    .map_err(write_error)?;
    replace_tags(&mut tx, id, &device.tags).await?;
    sqlx::query(
        "INSERT INTO devices.ssh_settings (device_id, enabled, port, username, auth) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(device.ssh.enabled)
    .bind(i32::from(device.ssh.port))
    .bind(&device.ssh.username)
    .bind(device.ssh.auth.as_str())
    .execute(&mut *tx)
    .await?;
    remote::write_new(&mut tx, id, &device.remote).await?;
    let created = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(created)
}

async fn replace_tags(conn: &mut PgConnection, id: DeviceId, tags: &[String]) -> Result<(), Error> {
    sqlx::query("DELETE FROM devices.device_tags WHERE device_id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("INSERT INTO devices.device_tags (device_id, tag) SELECT $1, unnest($2::text[])")
        .bind(id)
        .bind(tags)
        .execute(conn)
        .await?;
    Ok(())
}

/// Changes only the fields present; tags and each settings object are replaced wholly when present.
pub(super) async fn update_device(
    conn: &mut PgConnection,
    id: DeviceId,
    now: DateTime<Utc>,
    change: &DeviceChange,
) -> Result<Device, Error> {
    let mut tx = conn.begin().await?;
    let updated = sqlx::query(
        "UPDATE devices.devices SET site_id = COALESCE($2, site_id), name = COALESCE($3, name), \
                host = COALESCE($4, host), kind = COALESCE($5, kind), os = COALESCE($6, os), \
                maintenance = COALESCE($7, maintenance), updated_at = $8, \
                ip = CASE WHEN $4::text IS NULL THEN ip ELSE $9::inet END WHERE id = $1",
    )
    .bind(id)
    .bind(change.site)
    .bind(change.name.as_deref())
    .bind(change.host.as_deref())
    .bind(change.kind.as_deref())
    .bind(change.os.map(OperatingSystem::as_str))
    .bind(change.maintenance)
    .bind(now)
    .bind(change.ip())
    .execute(&mut *tx)
    .await
    .map_err(write_error)?;
    if updated.rows_affected() == 0 {
        return Err(Error::NotFound(Missing::Device(id)));
    }
    if let Some(tags) = &change.tags {
        replace_tags(&mut tx, id, tags).await?;
    }
    if let Some(ssh) = &change.ssh {
        sqlx::query(
            "UPDATE devices.ssh_settings SET enabled = $2, port = $3, username = $4, auth = $5 \
             WHERE device_id = $1",
        )
        .bind(id)
        .bind(ssh.enabled)
        .bind(i32::from(ssh.port))
        .bind(&ssh.username)
        .bind(ssh.auth.as_str())
        .execute(&mut *tx)
        .await?;
    }
    remote::write_change(&mut tx, id, &change.remote).await?;
    let ssh_off = change.ssh.as_ref().is_some_and(|ssh| !ssh.enabled);
    remote::release_relay(&mut tx, id, change.site, ssh_off).await?;
    let changed = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(changed)
}

/// Deletes with one `DELETE`; foreign keys remove tags and SSH settings (DEV-015).
/// The device is read `FOR UPDATE` first, so the returned device is the one deleted.
pub(super) async fn delete_device(conn: &mut PgConnection, id: DeviceId) -> Result<Device, Error> {
    let mut tx = conn.begin().await?;
    let row = sqlx::query_as::<_, DeviceRow>(select_device!("WHERE d.id = $1 FOR UPDATE OF d"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound(Missing::Device(id)))?;
    let device = Device::try_from(row)?;
    let deleted = sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(Error::NotFound(Missing::Device(id)));
    }
    tx.commit().await?;
    Ok(device)
}

pub(super) async fn fetch<'e>(db: impl PgExecutor<'e>, id: DeviceId) -> Result<Device, Error> {
    let row = sqlx::query_as::<_, DeviceRow>(select_device!("WHERE d.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound(Missing::Device(id)))?;
    Ok(Device::try_from(row)?)
}

pub(super) async fn get(db: &PgPool, id: DeviceId) -> Result<Device, Error> {
    fetch(db, id).await
}

/// Every device, in byte order of the name.
pub(super) async fn all(db: &PgPool) -> Result<Vec<Device>, Error> {
    let rows = sqlx::query_as(select_device!("ORDER BY d.name COLLATE \"C\""))
        .fetch_all(db)
        .await?;
    to_devices(rows)
}

/// One page of devices in byte order of the name.
#[derive(Debug)]
pub(super) struct DevicePage {
    pub(super) devices: Vec<Device>,
    pub(super) has_next_page: bool,
    /// The size of the filtered set; the cursor does not narrow it.
    pub(super) total: i64,
}

/// The page of filtered devices after the named one.
pub(super) async fn list(
    db: &PgPool,
    filter: &DeviceFilter,
    first: usize,
    after: Option<&str>,
) -> Result<DevicePage, Error> {
    let pattern = filter.name.as_deref().map(like_pattern);
    let limit = i64::try_from(first.saturating_add(1)).unwrap_or(i64::MAX);
    let rows = sqlx::query_as(select_device!(
        "WHERE ",
        device_filter!(),
        " AND ($4::text IS NULL OR d.name COLLATE \"C\" > $4) ORDER BY d.name COLLATE \"C\" LIMIT $5"
    ))
    .bind(&pattern)
    .bind(&filter.tags)
    .bind(filter.site)
    .bind(after)
    .bind(limit)
    .fetch_all(db)
    .await?;
    let total = sqlx::query_scalar(concat!(
        "SELECT count(*) FROM devices.devices d WHERE ",
        device_filter!()
    ))
    .bind(&pattern)
    .bind(&filter.tags)
    .bind(filter.site)
    .fetch_one(db)
    .await?;
    let page = split_page(to_devices(rows)?, first);
    Ok(DevicePage {
        devices: page.rows,
        has_next_page: page.has_next_page,
        total,
    })
}

/// Every device that passes the filter, in byte order of the name, with no paging.
pub(super) async fn filtered(db: &PgPool, filter: &DeviceFilter) -> Result<Vec<Device>, Error> {
    let rows = sqlx::query_as(select_device!(
        "WHERE ",
        device_filter!(),
        " ORDER BY d.name COLLATE \"C\""
    ))
    .bind(filter.name.as_deref().map(like_pattern))
    .bind(&filter.tags)
    .bind(filter.site)
    .fetch_all(db)
    .await?;
    to_devices(rows)
}

/// A substring pattern for `ILIKE` that matches `%`, `_` and `\` literally.
fn like_pattern(text: &str) -> String {
    let mut pattern = String::from("%");
    for character in text.chars() {
        if matches!(character, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

pub(super) async fn devices_by_sites(db: &PgPool, sites: &[SiteId]) -> Result<Vec<Device>, Error> {
    let rows = sqlx::query_as(select_device!(
        "WHERE d.site_id = ANY($1) ORDER BY d.name COLLATE \"C\""
    ))
    .bind(sites)
    .fetch_all(db)
    .await?;
    to_devices(rows)
}

pub(super) async fn in_maintenance(db: &PgPool, id: DeviceId) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT maintenance FROM devices.devices WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound(Missing::Device(id)))
}

/// Every tag in use, once, in byte order.
pub(super) async fn tags(db: &PgPool) -> Result<Vec<String>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT tag FROM (SELECT DISTINCT tag FROM devices.device_tags) t ORDER BY tag COLLATE \"C\"",
    )
    .fetch_all(db)
    .await?)
}
