//! Database records of the `devices` schema and how its constraints map to field errors.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnet::IpNet;

use super::{
    Error, FieldError,
    model::{Device, OperatingSystem, Site, SshAuth, SshSettings},
};
use crate::ids::{DeviceId, SiteId};

#[derive(sqlx::FromRow)]
pub(super) struct DeviceRow {
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

pub(super) fn to_devices(rows: Vec<DeviceRow>) -> Result<Vec<Device>, Error> {
    Ok(rows
        .into_iter()
        .map(Device::try_from)
        .collect::<Result<_, _>>()?)
}

pub(super) fn to_site((id, name, subnet): (SiteId, String, IpNet)) -> Site {
    Site { id, name, subnet }
}

/// Turns a violated constraint into the field the operator must fix.
pub(super) fn write_error(error: sqlx::Error) -> Error {
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
