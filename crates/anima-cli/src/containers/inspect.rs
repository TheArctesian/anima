//! `anima containers inspect <device> <container>`: the details of one container (HOST-017).
//! The hub gives the names of the environment variables and never a value. A command or a
//! label can hold a secret, so the hub refuses an API key (D-23) and no MCP tool returns the
//! details (MCP-004).

use super::documents::{ContainerInspect, container_inspect};
use super::logs::identifier;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

type Detail = container_inspect::ContainerInspectContainerDetail;

#[derive(Debug, clap::Args)]
pub struct InspectArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The container: its exact name, or 12 to 64 digits of its identifier.
    container: String,
}

impl InspectArgs {
    /// Prints a row for each detail or, with `--json`, the K1 `ContainerDetail`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device or container; the hub's refusal for an API
    /// key, or when the device cannot be read.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let container_id = identifier(&hub, &self.device, &self.container).await?;
        let variables = container_inspect::Variables {
            device_id: crate::devices::get::resolve(&hub, &self.device).await?,
            container_id,
        };
        let detail = hub
            .run::<ContainerInspect>(variables)
            .await?
            .container_detail;
        invocation.output.data(&detail, table)
    }
}

fn or_dash(text: Option<&str>) -> &str {
    text.filter(|text| !text.is_empty()).unwrap_or("-")
}

fn table(detail: &Detail) -> Table {
    let mut table = Table::record();
    table.pair("Name", detail.name.as_str());
    table.pair("ID", detail.id.as_str());
    table.pair("Image", detail.image.as_str());
    table.pair("Image ID", detail.image_id.as_str());
    table.pair("State", Cell::status(&enum_name(&detail.state)));
    table.pair("Exit code", detail.exit_code.to_string());
    table.pair("Started", or_dash(detail.started_at.as_deref()));
    table.pair("Finished", or_dash(detail.finished_at.as_deref()));
    table.pair("Restarts", detail.restart_count.to_string());
    table.pair("Restart policy", detail.restart_policy.as_str());
    table.pair("Command", detail.command.join(" "));
    table.pair("Working dir", or_dash(Some(&detail.working_dir)));
    table.pair("User", or_dash(Some(&detail.user)));
    health_rows(&mut table, detail);
    for mount in &detail.mounts {
        let access = if mount.read_only {
            "read-only"
        } else {
            "read-write"
        };
        let from = mount.name.as_deref().unwrap_or(&mount.source);
        let line = format!("{} {from} -> {} ({access})", mount.type_, mount.destination);
        table.pair("Mount", line);
    }
    for address in &detail.networks {
        let ips = [address.ipv4.as_deref(), address.ipv6.as_deref()];
        let ips: Vec<&str> = ips.into_iter().flatten().collect();
        table.pair("Network", format!("{} {}", address.network, ips.join(" ")));
    }
    for label in &detail.labels {
        table.pair("Label", format!("{}={}", label.name, label.value));
    }
    table.pair("Environment", detail.environment_names.join(", "));
    for process in &detail.processes {
        let user = process.user.as_deref().unwrap_or("-");
        table.pair(
            "Process",
            format!("{} {user} {}", process.pid, process.command),
        );
    }
    table
}

fn health_rows(table: &mut Table, detail: &Detail) {
    let Some(check) = &detail.health_check else {
        table.pair("Health check", "-");
        return;
    };
    table.pair("Health check", check.test.join(" "));
    table.pair("Health", enum_name(&detail.health));
    table.pair("Failing streak", detail.failing_streak.to_string());
    if let Some(run) = &detail.last_health_run {
        let ended = or_dash(run.ended_at.as_deref());
        table.pair(
            "Last health run",
            format!("exit {} at {ended}", run.exit_code),
        );
    }
}
