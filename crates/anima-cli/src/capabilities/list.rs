//! `anima capability list`: each device's tool inventory, filtered by a usable tool or a build
//! target (CAP-030). K1 has no pages for it.

use super::documents::{CapabilityList, capability_list};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only devices where this tool is usable, for example cargo or claude.
    #[arg(long)]
    tool: Option<String>,
    /// Only devices with this Rust build target, for example aarch64-apple-darwin.
    #[arg(long)]
    target: Option<String>,
}

impl ListArgs {
    /// Prints a row for each device or, with `--json`, the K1 `DeviceCapabilities` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let found = fetch(&hub, self.tool, self.target).await?;
        invocation.output.data(&found, table)
    }
}

/// The devices the hub lists for `tool` and `build_target`.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(
    hub: &Hub,
    tool: Option<String>,
    build_target: Option<String>,
) -> Result<Vec<capability_list::CapabilityListCapabilities>, Failure> {
    let variables = capability_list::Variables { tool, build_target };
    Ok(hub.run::<CapabilityList>(variables).await?.capabilities)
}

fn table(found: &Vec<capability_list::CapabilityListCapabilities>) -> Table {
    let mut table = Table::new(vec!["DEVICE", "SYSTEM", "TOOLS", "BUILD TARGETS"]);
    for row in found {
        let system = format!("{} {} {}", row.os, row.os_version, row.architecture);
        table.row(vec![
            row.device.name.as_str().into(),
            system.into(),
            Cell::new(&tools(&row.tools)),
            Cell::new(&targets(&row.build_targets)),
        ]);
    }
    table
}

/// `git 2.43.0, bun not usable`; `-` for none.
fn tools(tools: &[capability_list::CapabilityListCapabilitiesTools]) -> String {
    let shown: Vec<String> = tools
        .iter()
        .map(|tool| match &tool.version {
            Some(version) if tool.usable => format!("{} {version}", tool.name),
            _ => format!("{} not usable", tool.name),
        })
        .collect();
    or_dash(&shown.join(", "))
}

fn targets(targets: &[String]) -> String {
    or_dash(&targets.join(", "))
}

fn or_dash(text: &str) -> String {
    if text.is_empty() {
        "-".to_owned()
    } else {
        text.to_owned()
    }
}
