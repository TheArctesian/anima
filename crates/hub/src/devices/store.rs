//! All SQL on schema `devices`: I/O and row mapping only. Ids and time come in as parameters.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnet::IpNet;
use sqlx::{Connection, PgConnection, PgExecutor, PgPool};

use super::{
    Error, FieldError, Missing,
    model::{
        Device, DeviceChange, DeviceFilter, NewDevice, NewSite, OperatingSystem, Site, SshAuth,
        SshSettings,
    },
    page::split_page,
};
use crate::ids::{DeviceId, SiteId};

/// A device with its SSH settings and tags in one row.
macro_rules! select_device {
    ($($tail:expr),*) => {
        concat!(
            "SELECT d.id, d.site_id, d.name, d.host, d.ip, d.kind, d.os, d.maintenance, \
                    d.created_at, d.updated_at, s.enabled, s.port, s.username, s.auth, \
                    ARRAY(SELECT t.tag FROM devices.device_tags t WHERE t.device_id = d.id \
                          ORDER BY t.tag COLLATE \"C\") AS tags \
             FROM devices.devices d JOIN devices.ssh_settings s ON s.device_id = d.id ",
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

#[derive(sqlx::FromRow)]
struct DeviceRow {
    id: DeviceId,
    site_id: SiteId,
    name: String,
    host: String,
    ip: Option<IpAddr>,
    kind: String,
    os: String,
    maintenance: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    enabled: bool,
    port: i32,
    username: String,
    auth: String,
    tags: Vec<String>,
}

fn decode<E: std::error::Error + Send + Sync + 'static>(error: E) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

impl TryFrom<DeviceRow> for Device {
    type Error = sqlx::Error;

    fn try_from(row: DeviceRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.id,
            site: row.site_id,
            name: row.name,
            host: row.host,
            ip: row.ip,
            kind: row.kind,
            os: row.os.parse::<OperatingSystem>().map_err(decode)?,
            tags: row.tags,
            maintenance: row.maintenance,
            ssh: SshSettings {
                enabled: row.enabled,
                port: u16::try_from(row.port).map_err(decode)?,
                username: row.username,
                auth: row.auth.parse::<SshAuth>().map_err(decode)?,
            },
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn to_devices(rows: Vec<DeviceRow>) -> Result<Vec<Device>, Error> {
    Ok(rows
        .into_iter()
        .map(Device::try_from)
        .collect::<Result<_, _>>()?)
}

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
fn to_site((id, name, subnet): (SiteId, String, IpNet)) -> Site {
    Site { id, name, subnet }
}

/// Turns a violated constraint into the field the operator must fix.
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
fn write_error(error: sqlx::Error) -> Error {
    let field = match error.as_database_error().and_then(|e| e.constraint()) {
        Some("sites_name_key" | "devices_name_key") => {
            FieldError::new("name", "That name is already taken. Choose another.")
        }
        Some("sites_subnet_key" | "sites_subnet_no_overlap") => {
            FieldError::new("subnet", "This subnet overlaps an existing site subnet.")
        }
        Some("devices_site_id_fkey") => FieldError::new("siteId", "There is no such site."),
        _ => return Error::Database(error),
    };
    Error::Validation(vec![field])
}

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn create_site(
    conn: &mut PgConnection,
    id: SiteId,
    site: &NewSite,
) -> Result<Site, Error> {
    sqlx::query_as("INSERT INTO devices.sites (id, name, subnet) VALUES ($1, $2, $3) RETURNING id, name, subnet")
        .bind(id)
        .bind(&site.name)
        .bind(site.subnet)
        .fetch_one(conn)
        .await
        .map(to_site)
        .map_err(write_error)
}

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn list_sites(db: &PgPool) -> Result<Vec<Site>, Error> {
    let rows =
        sqlx::query_as("SELECT id, name, subnet FROM devices.sites ORDER BY name COLLATE \"C\"")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(to_site).collect())
}

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn sites_by_ids(db: &PgPool, ids: &[SiteId]) -> Result<Vec<Site>, Error> {
    let rows = sqlx::query_as(
        "SELECT id, name, subnet FROM devices.sites WHERE id = ANY($1) ORDER BY name COLLATE \"C\"",
    )
    .bind(ids)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(to_site).collect())
}

/// Inserts the device, its tags and its SSH settings; all or nothing.
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn create_device(
    conn: &mut PgConnection,
    id: DeviceId,
    now: DateTime<Utc>,
    device: &NewDevice,
) -> Result<Device, Error> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
    )
    .bind(id)
    .bind(device.site)
    .bind(&device.name)
    .bind(&device.host)
    .bind(&device.kind)
    .bind(device.os.as_str())
    .bind(now)
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
    let created = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(created)
}

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
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

/// Changes only the fields present; tags and SSH settings are replaced wholly when present.
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
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
                maintenance = COALESCE($7, maintenance), updated_at = $8 WHERE id = $1",
    )
    .bind(id)
    .bind(change.site)
    .bind(change.name.as_deref())
    .bind(change.host.as_deref())
    .bind(change.kind.as_deref())
    .bind(change.os.map(OperatingSystem::as_str))
    .bind(change.maintenance)
    .bind(now)
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
    let changed = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(changed)
}

/// Deletes with one `DELETE`; foreign keys remove tags and SSH settings (DEV-015).
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn delete_device(conn: &mut PgConnection, id: DeviceId) -> Result<Device, Error> {
    let device = fetch(&mut *conn, id).await?;
    let deleted = sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(Error::NotFound(Missing::Device(id)));
    }
    Ok(device)
}

async fn fetch<'e>(db: impl PgExecutor<'e>, id: DeviceId) -> Result<Device, Error> {
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

/// One page of the filtered devices in byte order of the name, whether more follow, and the
/// size of the filtered set (the cursor does not narrow it).
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn list(
    db: &PgPool,
    filter: &DeviceFilter,
    first: usize,
    after: Option<&str>,
) -> Result<(Vec<Device>, bool, i64), Error> {
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
    Ok((page.rows, page.has_next_page, total))
}

/// A substring pattern for `ILIKE` that matches `%`, `_` and `\` literally.
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
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

#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
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
#[cfg_attr(not(test), expect(dead_code, reason = "used by devices GraphQL"))]
pub(super) async fn tags(db: &PgPool) -> Result<Vec<String>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT tag FROM (SELECT DISTINCT tag FROM devices.device_tags) t ORDER BY tag COLLATE \"C\"",
    )
    .fetch_all(db)
    .await?)
}
