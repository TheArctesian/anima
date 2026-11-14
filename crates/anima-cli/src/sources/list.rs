//! `anima source list`.

use super::documents::{SourceAccounts, source_accounts};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only the accounts of this source, such as github.
    #[arg(long)]
    source: Option<String>,
}

impl ListArgs {
    /// Prints the accounts as a table or, with `--json`, as the K1 `SourceAccount` list with
    /// each account's last run.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = source_accounts::Variables {
            source_key: self.source,
            runs: Some(1),
        };
        let answer = hub.run::<SourceAccounts>(variables).await?;
        invocation
            .output
            .data(&answer.source_accounts, render::list)
    }
}
