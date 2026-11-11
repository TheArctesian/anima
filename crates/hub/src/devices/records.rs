//! Database records of the `devices` schema and how its constraints map to field errors.

use std::{net::IpAddr, num::NonZeroU8};

use chrono::{DateTime, Utc};
use ipnet::IpNet;

use super::{
    Error, FieldError,
    mac::MacAddress,
    model::{Device, Distribution, OperatingSystem, Site, SshAuth, SshSettings, UnknownValue},
    remote::{RdpSettings, RemoteAccess, TransferPolicy, VncSettings},
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
    distribution: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    enabled: bool,
    port: i32,
    username: String,
    auth: String,
    tags: Vec<String>,
    #[sqlx(flatten)]
    remote: RemoteRow,
}

/// The remote access columns of a device, joined from `rdp_settings`, `vnc_settings` and
/// `devices`. The address is read as text.
#[derive(sqlx::FromRow)]
#[expect(clippy::struct_excessive_bools, reason = "one column per switch")]
pub(super) struct RemoteRow {
    rdp_enabled: bool,
    rdp_port: i32,
    rdp_username: String,
    rdp_domain: Option<String>,
    rdp_security: String,
    rdp_ignore_certificate: bool,
    rdp_resize_method: String,
    rdp_audio_out: bool,
    rdp_microphone: bool,
    rdp_shared_drive: bool,
    vnc_enabled: bool,
    vnc_port: i32,
    vnc_username: Option<String>,
    mac_address: Option<String>,
    metrics_source: String,
    session_limit: Option<i32>,
    record_sessions: bool,
    transfer_upload: bool,
    transfer_download: bool,
    transfer_copy: bool,
    transfer_paste: bool,
}

fn decode<E: std::error::Error + Send + Sync + 'static>(error: E) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

/// The stored `session_limit` column: null is no limit (RA-036).
pub(super) fn session_limit(stored: Option<i32>) -> Result<Option<NonZeroU8>, sqlx::Error> {
    stored
        .map(|limit| {
            u8::try_from(limit)
                .map_err(decode)?
                .try_into()
                .map_err(decode)
        })
        .transpose()
}

impl TryFrom<RemoteRow> for RemoteAccess {
    type Error = sqlx::Error;

    fn try_from(row: RemoteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            rdp: RdpSettings {
                enabled: row.rdp_enabled,
                port: u16::try_from(row.rdp_port).map_err(decode)?,
                username: row.rdp_username,
                domain: row.rdp_domain,
                security: row.rdp_security.parse().map_err(decode)?,
                ignore_certificate: row.rdp_ignore_certificate,
                resize: row.rdp_resize_method.parse().map_err(decode)?,
                audio_out: row.rdp_audio_out,
                microphone: row.rdp_microphone,
                shared_drive: row.rdp_shared_drive,
            },
            vnc: VncSettings {
                enabled: row.vnc_enabled,
                port: u16::try_from(row.vnc_port).map_err(decode)?,
                username: row.vnc_username,
            },
            mac_address: row
                .mac_address
                .map(|text| text.parse::<MacAddress>())
                .transpose()
                .map_err(decode)?,
            metrics_source: row.metrics_source.parse().map_err(decode)?,
            session_limit: session_limit(row.session_limit)?,
            record_sessions: row.record_sessions,
            transfer: TransferPolicy {
                upload: row.transfer_upload,
                download: row.transfer_download,
                copy: row.transfer_copy,
                paste: row.transfer_paste,
            },
        })
    }
}

impl TryFrom<DeviceRow> for Device {
    type Error = sqlx::Error;

    fn try_from(row: DeviceRow) -> Result<Self, sqlx::Error> {
        let remote = RemoteAccess::try_from(row.remote)?;
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
            rdp: remote.rdp,
            vnc: remote.vnc,
            mac_address: remote.mac_address,
            metrics_source: remote.metrics_source,
            session_limit: remote.session_limit,
            record_sessions: remote.record_sessions,
            transfer: remote.transfer,
            distribution: row.distribution.as_deref().map(distribution).transpose()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// The stored distribution; the column's check keeps it valid, so anything else is a decode error.
fn distribution(text: &str) -> Result<Distribution, sqlx::Error> {
    Distribution::new(text).ok_or_else(|| decode(UnknownValue::new("distribution", text)))
}

pub(super) fn to_devices(rows: Vec<DeviceRow>) -> Result<Vec<Device>, Error> {
    Ok(rows
        .into_iter()
        .map(Device::try_from)
        .collect::<Result<_, _>>()?)
}

pub(super) fn to_site(
    (id, name, subnet, wake_relay): (SiteId, String, IpNet, Option<DeviceId>),
) -> Site {
    Site {
        id,
        name,
        subnet,
        wake_relay,
    }
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
