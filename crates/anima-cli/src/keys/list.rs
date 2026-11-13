//! `anima key list`.

use super::documents::{KeyList, key_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct ListArgs;

impl ListArgs {
    /// Prints the keys as a table or, with `--json`, as the K1 `ApiKey` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<KeyList>(key_list::Variables).await?;
        invocation.output.data(&answer.api_keys, table)
    }
}

fn table(keys: &Vec<key_list::KeyListApiKeys>) -> Table {
    let mut table = Table::new(vec![
        "NAME",
        "SCOPE",
        "PREFIX",
        "EXPIRES",
        "LAST USED",
        "ID",
    ]);
    for key in keys {
        table.row(vec![
            key.name.as_str().into(),
            enum_name(&key.scope).into(),
            key.prefix.as_str().into(),
            key.expires_at.as_str().into(),
            key.last_used_at.as_deref().unwrap_or("never").into(),
            key.id.as_str().into(),
        ]);
    }
    table
}
