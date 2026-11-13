//! `anima device get <id-or-name>`: one device, found by ID or by its exact name.

use super::documents::{DeviceGet, device_get, device_list};
use super::list::{MAX_PAGE, Query, fetch};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl GetArgs {
    /// Prints the device as label and value lines or, with `--json`, as the K1 `Device`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when no device has the ID or name; [`Failure::Usage`] when more
    /// than one device has the name; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let answer = hub
            .run::<DeviceGet>(device_get::Variables { id: id.clone() })
            .await?;
        let device = answer.device.ok_or_else(|| Failure::NotFound {
            message: format!("No device has the ID {id}."),
            hint: Some("anima device list".to_owned()),
        })?;
        invocation.output.data(&device, table)
    }
}

/// The ID `device` names: an ID as it is, or the one device with exactly that name.
///
/// # Errors
/// [`Failure::NotFound`] when no device has the name; [`Failure::Usage`], listing them, when
/// more than one has it.
pub async fn resolve(hub: &Hub, device: &str) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(device).is_ok() {
        return Ok(device.to_owned());
    }
    // The hub's name filter is a substring in any case; only an exact match counts.
    let query = Query {
        filter: device_list::DeviceFilter {
            name: Some(device.to_owned()),
            status: None,
            tags: None,
            site_id: None,
        },
        sort: device_list::DeviceSort::NAME,
        first: MAX_PAGE,
    };
    let candidates = fetch(hub, &query, None, true).await?;
    let mut matches: Vec<_> = candidates
        .edges
        .into_iter()
        .map(|edge| edge.node)
        .filter(|node| node.name == device)
        .collect();
    match matches.len() {
        0 => Err(Failure::NotFound {
            message: format!("No device is named {device}."),
            hint: Some(format!("anima device list --name {device}")),
        }),
        1 => Ok(matches.remove(0).id),
        count => {
            let listed: Vec<String> = matches
                .iter()
                .map(|node| format!("{} (site {})", node.id, node.site.name))
                .collect();
            Err(Failure::Usage {
                message: format!(
                    "{count} devices are named {device}: {}. Give the ID.",
                    listed.join(", ")
                ),
                hint: Some(format!("anima device get {}", matches[0].id)),
            })
        }
    }
}

fn table(device: &device_get::DeviceGetDevice) -> Table {
    let mut table = Table::record();
    table.pair("Name", device.name.as_str());
    table.pair("ID", device.id.as_str());
    table.pair("Status", Cell::status(&enum_name(&device.status)));
    table.pair("Site", device.site.name.as_str());
    table.pair("Host", device.host.as_str());
    table.pair("IP", device.ip.as_deref().unwrap_or("-"));
    table.pair("Kind", device.kind.as_str());
    table.pair("OS", system(device));
    table.pair("Tags", device.tags.join(", "));
    table.pair("Maintenance", if device.maintenance { "on" } else { "off" });
    table.pair(
        "SSH",
        access(
            device.ssh.enabled,
            device.ssh.port,
            device.ssh.credential_stored,
        ),
    );
    table.pair(
        "RDP",
        access(
            device.rdp.enabled,
            device.rdp.port,
            device.rdp.credential_stored,
        ),
    );
    table.pair(
        "VNC",
        access(
            device.vnc.enabled,
            device.vnc.port,
            device.vnc.credential_stored,
        ),
    );
    if let Some(key) = &device.ssh.host_key {
        table.pair(
            "Host key",
            format!("{} {}", key.algorithm, key.fingerprint_sha256),
        );
    }
    table.pair("Metrics", enum_name(&device.metrics_source));
    table.pair(
        "Uptime 24 h",
        device
            .uptime
            .map_or_else(|| "-".to_owned(), |ratio| format!("{:.2} %", ratio * 100.0)),
    );
    for check in &device.checks {
        let label = format!("Check {} ({})", check.name, enum_name(&check.type_));
        let status = if check.paused {
            "PAUSED".to_owned()
        } else {
            enum_name(&check.status)
        };
        table.pair(&label, Cell::status(&status));
    }
    table.pair("Updated", device.updated_at.as_str());
    table
}

fn system(device: &device_get::DeviceGetDevice) -> String {
    let os = enum_name(&device.os);
    match &device.distribution {
        Some(distribution) => format!("{os} ({distribution})"),
        None => os,
    }
}

fn access(enabled: bool, port: i64, credential_stored: bool) -> String {
    if !enabled {
        return "off".to_owned();
    }
    let credential = if credential_stored {
        "credential stored"
    } else {
        "no credential"
    };
    format!("port {port}, {credential}")
}
