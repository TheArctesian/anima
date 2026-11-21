//! `anima capability list`: each device's tool inventory and hardware values, filtered by a
//! usable tool or a build target (CAP-030, CAP-032). K1 has no pages for it.

use super::documents::{CapabilityList, capability_list};
use super::hint::{backend_hint, backend_repairable, hint};
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
        invocation.output.data(&found, table)?;
        for note in hints(&found) {
            invocation.output.note(&note);
        }
        Ok(())
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
    let mut table = Table::new(vec![
        "DEVICE",
        "SYSTEM",
        "HARDWARE",
        "TOOLS",
        "BUILD TARGETS",
    ]);
    for row in found {
        let system = format!("{} {} {}", row.os, row.os_version, row.architecture);
        table.row(vec![
            row.device.name.as_str().into(),
            system.into(),
            Cell::new(
                &row.hardware
                    .as_ref()
                    .map_or_else(|| "-".to_owned(), hardware),
            ),
            Cell::new(&tools(&row.tools)),
            Cell::new(&targets(&row.build_targets)),
        ]);
    }
    table
}

/// `git 2.43.0, bun not usable (timed out)`; `-` for none.
fn tools(tools: &[capability_list::CapabilityListCapabilitiesTools]) -> String {
    let shown: Vec<String> = tools
        .iter()
        .map(|tool| match (&tool.version, &tool.reason) {
            (Some(version), _) if tool.usable => format!("{} {version}", tool.name),
            (_, Some(reason)) => format!("{} not usable ({})", tool.name, reason_text(reason)),
            _ => format!("{} not usable", tool.name),
        })
        .collect();
    or_dash(&shown.join(", "))
}

/// `48 GiB memory, 48 GiB unified, 36 GiB usable (default share), 390 GiB free; backends
/// metal, cpu; mains, nominal, active`.
fn hardware(values: &capability_list::CapabilityListCapabilitiesHardware) -> String {
    use capability_list::UsableMemorySource as Source;
    let mut memory = vec![format!("{} memory", gib(values.system_memory_bytes))];
    if let Some(bytes) = values.unified_memory_bytes {
        memory.push(format!("{} unified", gib(bytes)));
    }
    if let Some(bytes) = values.dedicated_memory_bytes {
        memory.push(format!("{} dedicated", gib(bytes)));
    }
    if let Some(bytes) = values.usable_memory_bytes {
        let source = match &values.usable_memory_source {
            Some(Source::WIRED_LIMIT) => " (wired limit)",
            Some(Source::DEFAULT_SHARE) => " (default share)",
            Some(Source::Other(_)) | None => "",
        };
        memory.push(format!("{} usable{source}", gib(bytes)));
    }
    if let Some(bytes) = values.free_disk_bytes {
        memory.push(format!("{} free", gib(bytes)));
    }
    let usable: Vec<&str> = values
        .backends
        .iter()
        .filter(|backend| backend.usable)
        .map(|backend| backend.name.as_str())
        .collect();
    let mut state = Vec::new();
    state.extend(
        values
            .power_source
            .as_ref()
            .map(|source| format!("{source:?}").to_lowercase()),
    );
    state.extend(
        values
            .thermal_state
            .as_ref()
            .map(|state| format!("{state:?}").to_lowercase()),
    );
    state.extend(
        values
            .operator_active
            .map(|active| if active { "active" } else { "idle" }.to_owned()),
    );
    format!(
        "{}; backends {}; {}",
        memory.join(", "),
        or_dash(&usable.join(", ")),
        or_dash(&state.join(", "))
    )
}

/// Whole GiB, rounded down: `36 GiB`.
fn gib(bytes: f64) -> String {
    format!("{:.0} GiB", (bytes / 1_073_741_824.0).floor())
}

/// `pve-01: bun: <hint>` for each tool that is not usable, and `pve-01: rocm: <hint>` for each
/// backend the operator can repair, for standard error.
fn hints(found: &[capability_list::CapabilityListCapabilities]) -> Vec<String> {
    let mut notes = Vec::new();
    for row in found {
        let backends = row.hardware.iter().flat_map(|values| &values.backends);
        for backend in backends {
            if let Some(reason) = backend.reason.as_ref().filter(|r| backend_repairable(r)) {
                let name = &backend.name;
                notes.push(format!(
                    "{}: {name}: {}",
                    row.device.name,
                    backend_hint(name, reason)
                ));
            }
        }
        for tool in &row.tools {
            if let Some(reason) = &tool.reason {
                notes.push(format!(
                    "{}: {}: {}",
                    row.device.name,
                    tool.name,
                    hint(&tool.name, reason)
                ));
            }
        }
    }
    notes
}

fn reason_text(reason: &capability_list::NotUsableReason) -> &'static str {
    use capability_list::NotUsableReason as Reason;
    match reason {
        Reason::TIMED_OUT => "timed out",
        Reason::EXITED => "exited",
        Reason::NO_VERSION => "no version",
        Reason::FAILED | Reason::Other(_) => "failed",
    }
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
