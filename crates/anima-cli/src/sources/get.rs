//! `anima source get <id-or-label>`: one account with its runs and their counts (ITG-003).

use super::account::resolve;
use super::documents::{SourceAccounts, source_accounts};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;

/// The most runs the hub returns for an account (ITG-003).
const RUNS: i64 = 20;

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The account's ID or its exact label.
    account: String,
}

impl GetArgs {
    /// Prints the account and its runs or, with `--json`, the K1 `SourceAccount` with `runs`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let named = resolve(&hub, &self.account).await?;
        let variables = source_accounts::Variables {
            source_key: None,
            runs: Some(RUNS),
        };
        let accounts = hub.run::<SourceAccounts>(variables).await?.source_accounts;
        let account = accounts
            .into_iter()
            .find(|account| account.id == named.id)
            .ok_or_else(|| Failure::NotFound {
                message: format!("The source account {} is gone.", named.label),
                hint: None,
            })?;
        invocation.output.data(&account, render::record)
    }
}
