//! `anima source watch [<id-or-label>]`: one line for each change of a source account (K1
//! `sourceAccountChanged`).

use super::account::resolve;
use super::documents::{SourceAccountChanged, source_account_changed};
use super::render;
use crate::failure::Failure;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};

#[derive(Debug, clap::Args)]
pub struct WatchArgs {
    /// Only this account, by ID or exact label; without it, every account.
    account: Option<String>,
}

impl WatchArgs {
    /// Prints each change as a line or, with `--json`, as one K1 `SourceAccount` per line.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; [`Failure`] when the hub refuses the token
    /// or the connection is lost.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = match &self.account {
            Some(account) => Some(resolve(&hub, account).await?.id),
            None => None,
        };
        let variables = source_account_changed::Variables { id };
        let mut changes = Subscription::open::<SourceAccountChanged>(&hub, variables).await?;
        while let Some(answer) = changes.next().await {
            let account = answer?.source_account_changed;
            if stream::emit(invocation.output.json(), &account, render::change_line)?
                == Reader::Gone
            {
                break;
            }
        }
        Ok(())
    }
}
