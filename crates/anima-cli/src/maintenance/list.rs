//! `anima maintenance list`, and the lookup of a window by ID or exact name.

use super::documents::{MaintenanceList, maintenance_list};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

/// One window as `maintenanceWindows` returns it.
pub type Window = maintenance_list::MaintenanceListMaintenanceWindows;

#[derive(Debug, clap::Args)]
pub struct ListArgs;

impl ListArgs {
    /// Prints the windows as a table or, with `--json`, as K1 `[MaintenanceWindow]`.
    ///
    /// # Errors
    /// The hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let windows = hub
            .run::<MaintenanceList>(maintenance_list::Variables)
            .await?
            .maintenance_windows;
        invocation.output.data(&windows, |windows| table(windows))
    }
}

/// The window with this ID or exact name.
///
/// # Errors
/// [`Failure::NotFound`] when none has it; [`Failure::Usage`] when more than one has the name.
pub async fn find(hub: &Hub, window: &str) -> Result<Window, Failure> {
    let windows = hub
        .run::<MaintenanceList>(maintenance_list::Variables)
        .await?
        .maintenance_windows;
    let mut matches: Vec<Window> = windows
        .into_iter()
        .filter(|candidate| candidate.id == window || candidate.name == window)
        .collect();
    match matches.len() {
        0 => Err(Failure::NotFound {
            message: format!("No maintenance window has the ID or name {window}."),
            hint: Some(Hint::run("anima maintenance list")),
        }),
        1 => Ok(matches.remove(0)),
        count => Err(Failure::Usage {
            message: format!("{count} maintenance windows are named {window}. Give the ID."),
            hint: Some(Hint::run("anima maintenance list")),
        }),
    }
}

fn table(windows: &[Window]) -> Table {
    let mut table = Table::new(vec!["NAME", "KIND", "NOW", "SCHEDULE", "TARGETS", "ID"]);
    for window in windows {
        let targets: Vec<String> = window
            .targets
            .iter()
            .map(|target| {
                format!(
                    "{}:{}",
                    enum_name(&target.type_).to_lowercase(),
                    target.target
                )
            })
            .collect();
        let now = if window.active_now {
            Cell::toned("ACTIVE", Tone::Warning)
        } else {
            Cell::toned("idle", Tone::Muted)
        };
        table.row(vec![
            window.name.as_str().into(),
            enum_name(&window.kind).into(),
            now,
            schedule(window).into(),
            targets.join(",").into(),
            window.id.as_str().into(),
        ]);
    }
    table
}

/// The schedule in words, with its time zone.
fn schedule(window: &Window) -> String {
    let zone = &window.timezone;
    let at = window.start_time.as_deref().unwrap_or("?");
    let minutes = window
        .duration_minutes
        .map_or_else(String::new, |minutes| format!(" for {minutes} min"));
    match enum_name(&window.kind).as_str() {
        "MANUAL" => format!(
            "manual, {}",
            if window.active_manual == Some(true) {
                "on"
            } else {
                "off"
            }
        ),
        "SINGLE" => format!(
            "{} to {}",
            window.starts_at.as_deref().unwrap_or("?"),
            window.ends_at.as_deref().unwrap_or("?")
        ),
        "RECURRING_INTERVAL" => format!(
            "every {} days from {}{minutes}",
            window.every_days.unwrap_or_default(),
            window.first_start_at.as_deref().unwrap_or("?")
        ),
        "RECURRING_WEEKDAY" => {
            let days: Vec<String> = window.weekdays.iter().flatten().map(enum_name).collect();
            format!("{} at {at}{minutes} ({zone})", days.join(","))
        }
        _ => {
            let days: Vec<String> = window
                .days_of_month
                .iter()
                .flatten()
                .map(ToString::to_string)
                .collect();
            format!("day {} at {at}{minutes} ({zone})", days.join(","))
        }
    }
}
