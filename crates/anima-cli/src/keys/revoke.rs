//! `anima key revoke <id-or-name>`: the key stops working at once.

use super::documents::{KeyList, KeyRevoke, key_list, key_revoke};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RevokeArgs {
    /// The key's ID or its exact name.
    key: String,
}

impl RevokeArgs {
    /// Asks to confirm (or takes `--yes`, CMD-024), revokes the key and prints its ID.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown key; [`Failure::Usage`] without a confirmation
    /// when no one can be asked; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.key).await?;
        if !invocation
            .prompter
            .confirm(&format!("Revoke API key {}?", self.key))?
        {
            invocation.output.note("Nothing was revoked.");
            return Ok(());
        }
        let answer = hub.run::<KeyRevoke>(key_revoke::Variables { id }).await?;
        invocation.output.data(&answer, |answer| {
            let mut table = Table::record();
            table.pair("Revoked", answer.revoke_api_key.as_str());
            table
        })
    }
}

/// The ID `key` names: an ID as it is, or the one key with exactly that name.
async fn resolve(hub: &Hub, key: &str) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(key).is_ok() {
        return Ok(key.to_owned());
    }
    let keys = hub.run::<KeyList>(key_list::Variables).await?.api_keys;
    keys.into_iter()
        .find(|candidate| candidate.name == key)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("There is no API key named {key}."),
            hint: Some("anima key list".to_owned()),
        })
}
