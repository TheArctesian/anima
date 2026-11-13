//! `anima builder revoke <id-or-name>`: deletes a builder and revokes its token (BLD-025).

use super::documents::{BuilderList, BuilderRevoke, builder_list, builder_revoke};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RevokeArgs {
    /// The builder's ID or its exact name.
    builder: String,
}

impl RevokeArgs {
    /// Asks for confirmation (`--yes`), then revokes and prints the ID.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when no builder has the ID or name; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.builder).await?;
        invocation.prompter.confirm(&format!(
            "Revoke builder {}? Its token stops working.",
            self.builder
        ))?;
        let answer = hub
            .run::<BuilderRevoke>(builder_revoke::Variables { id })
            .await?;
        invocation.output.data(&answer, table)
    }
}

/// The ID of the builder `builder` names: an ID or exact name from `builders`.
async fn resolve(hub: &Hub, builder: &str) -> Result<String, Failure> {
    let builders = hub
        .run::<BuilderList>(builder_list::Variables)
        .await?
        .builders;
    builders
        .into_iter()
        .find(|candidate| candidate.id == builder || candidate.name == builder)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("No builder has the ID or name {builder}."),
            hint: Some(Hint::run("anima builder list")),
        })
}

fn table(answer: &builder_revoke::ResponseData) -> Table {
    let mut table = Table::record();
    table.pair("Revoked", answer.revoke_builder.as_str());
    table
}
