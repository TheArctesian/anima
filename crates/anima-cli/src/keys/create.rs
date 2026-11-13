//! `anima key create`: makes an API key and prints its secret. This is the one place the
//! secret is shown (INT-003); the hub keeps only its hash.

use chrono::Utc;
use serde_json::json;

use super::documents::{KeyCreate, key_create};
use super::expiry;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// A name for the key, 1 to 64 characters and unique.
    #[arg(long)]
    name: String,
    /// What the key may do: read, or read and change.
    #[arg(long, value_enum, default_value_t = ScopeArg::Read)]
    scope: ScopeArg,
    /// When the key stops working: a number of days (90d) or a time (2027-01-31T00:00:00Z),
    /// at most a year from now.
    #[arg(long)]
    expires: String,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ScopeArg {
    Read,
    Write,
}

impl CreateArgs {
    /// Prints the key and its secret, with a note on standard error that the secret is shown
    /// one time. With `--json`, the K1 `ApiKeyCreated`.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an unusable `--expires`; the hub's refusal (`VALIDATION`,
    /// `CONFLICT` for a name in use, `FORBIDDEN` for an API key).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let expires_at =
            expiry::parse(&self.expires, Utc::now()).map_err(|error| Failure::Usage {
                message: error.to_string(),
                hint: None,
            })?;
        let hub = invocation.hub()?;
        let input = key_create::ApiKeyInput {
            name: self.name,
            scope: self.scope.k1(),
            expires_at,
        };
        let created = hub
            .run::<KeyCreate>(key_create::Variables { input })
            .await?
            .create_api_key;
        let key = &created.key;
        let record = json!({
            "key": {
                "id": key.id, "name": key.name, "scope": enum_name(&key.scope),
                "prefix": key.prefix, "expiresAt": key.expires_at,
                "createdAt": key.created_at, "lastUsedAt": key.last_used_at,
            },
            "secret": created.secret,
        });
        invocation.output.data(&record, |record| {
            let mut table = Table::record();
            for (label, field) in [
                ("Name", "name"),
                ("Scope", "scope"),
                ("Expires", "expiresAt"),
                ("ID", "id"),
            ] {
                table.pair(label, record["key"][field].as_str().unwrap_or_default());
            }
            table.pair("Secret", record["secret"].as_str().unwrap_or_default());
            table
        })?;
        invocation
            .output
            .note("This secret is shown one time. Store it now; the hub keeps only its hash.");
        Ok(())
    }
}

impl ScopeArg {
    const fn k1(self) -> key_create::TokenScope {
        match self {
            Self::Read => key_create::TokenScope::READ,
            Self::Write => key_create::TokenScope::WRITE,
        }
    }
}
