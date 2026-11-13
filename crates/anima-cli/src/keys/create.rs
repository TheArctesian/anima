//! `anima key create`: makes an API key and prints its secret. This is the one place the
//! secret is shown (INT-003); the hub keeps only its hash.

use chrono::Utc;
use serde_json::json;

use super::documents::{KeyCreate, key_create};
use super::expiry;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::one_time_secret::{Secret, ToFile, show_once};
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// A name for the key, 1 to 64 characters and unique.
    #[arg(long)]
    name: String,
    /// What the key may do: read, or read and change.
    #[arg(long, value_enum, default_value_t = ScopeArg::Read)]
    scope: ScopeArg,
    /// Who uses the key: a script you run (user) or an AI agent (agent). The hub records each
    /// change made with an agent key as the work of an AI agent. It cannot change later.
    #[arg(long = "for", value_enum)]
    made_for: MadeForArg,
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

/// K1 `MadeFor` (LOG-015).
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum MadeForArg {
    User,
    Agent,
}

impl CreateArgs {
    /// Shows the secret alone on standard output, with the key and the note that the secret is
    /// shown one time on standard error. With `--json`, the K1 `ApiKeyCreated`.
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
            made_for: self.made_for.k1(),
            expires_at,
        };
        let created = hub
            .run::<KeyCreate>(key_create::Variables { input })
            .await?
            .create_api_key;
        let key = &created.key;
        invocation.output.note(&format!(
            "API key {} ({}, for {}), id {}, expires {}. The hub keeps only its hash.",
            key.name,
            enum_name(&key.scope),
            enum_name(&key.made_for),
            key.id,
            key.expires_at
        ));
        let public = json!({
            "key": {
                "id": key.id, "name": key.name, "scope": enum_name(&key.scope),
                "madeFor": enum_name(&key.made_for),
                "prefix": key.prefix, "expiresAt": key.expires_at,
                "createdAt": key.created_at, "lastUsedAt": key.last_used_at,
            },
        });
        let secret = Secret {
            label: "API key secret",
            field: "secret",
            value: &created.secret,
            to_file: &ToFile::default(),
        };
        show_once(invocation, &secret, public)
    }
}

impl MadeForArg {
    const fn k1(self) -> key_create::MadeFor {
        match self {
            Self::User => key_create::MadeFor::USER,
            Self::Agent => key_create::MadeFor::AGENT,
        }
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
