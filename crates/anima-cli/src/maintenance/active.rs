//! `anima maintenance start` and `anima maintenance stop`: a manual window on or off.

use super::documents::{MaintenanceSetActive, maintenance_set_active};
use super::list::find;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct StartArgs {
    /// The manual window's ID or exact name.
    window: String,
}

#[derive(Debug, clap::Args)]
pub struct StopArgs {
    /// The manual window's ID or exact name.
    window: String,
}

impl StartArgs {
    /// Turns the window on and prints it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown window; the hub's refusal (VALIDATION when the
    /// window is not manual).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_active(&self.window, true, invocation).await
    }
}

impl StopArgs {
    /// Turns the window off and prints it.
    ///
    /// # Errors
    /// As [`StartArgs::run`].
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_active(&self.window, false, invocation).await
    }
}

async fn set_active(window: &str, active: bool, invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let id = find(&hub, window).await?.id;
    let answer = hub
        .run::<MaintenanceSetActive>(maintenance_set_active::Variables { id, active })
        .await?;
    invocation
        .output
        .data(&answer.set_manual_window_active, |window| {
            let mut table = Table::record();
            table.pair("Name", window.name.as_str());
            table.pair("ID", window.id.as_str());
            table.pair("Kind", enum_name(&window.kind));
            table.pair("Active", if window.active_now { "yes" } else { "no" });
            table
        })
}
