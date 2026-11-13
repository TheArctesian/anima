//! `anima check list --device <id-or-name>`: the checks of one device with their key settings.

use super::documents::{CheckList, check_list};
use crate::checks::settings::one_line;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The device whose checks to list, by ID or exact name.
    #[arg(long)]
    device: String,
}

impl ListArgs {
    /// Prints the checks as a table or, with `--json`, as the K1 `Device` with its checks.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let answer = hub
            .run::<CheckList>(check_list::Variables { device: id.clone() })
            .await?;
        let device = answer.device.ok_or_else(|| Failure::NotFound {
            message: format!("No device has the ID {id}."),
            hint: Some("anima device list".to_owned()),
        })?;
        invocation.output.data(&device, table)
    }
}

fn table(device: &check_list::CheckListDevice) -> Table {
    let mut table = Table::new(vec!["NAME", "KIND", "STATUS", "EVERY", "SETTINGS", "ID"]);
    for check in &device.checks {
        let status = if check.paused {
            "PAUSED".to_owned()
        } else {
            enum_name(&check.status)
        };
        let config = serde_json::to_value(&check.config).unwrap_or_default();
        table.row(vec![
            check.name.as_str().into(),
            enum_name(&check.type_).into(),
            Cell::status(&status),
            format!("{} s", check.interval_seconds).into(),
            one_line(&config).into(),
            check.id.as_str().into(),
        ]);
    }
    table
}
