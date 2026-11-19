//! `anima location token add|list|revoke`: the ingest tokens of the OwnTracks app. A token is
//! a secret: `add` shows it one time, and no MCP tool makes or returns one (CMD-014).

use serde_json::json;
use zeroize::Zeroizing;

use super::documents::{
    LocationTokenAdd, LocationTokenList, LocationTokenRevoke, location_token_add,
    location_token_list, location_token_revoke,
};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::one_time_secret::{Secret, ToFile, Works, show_once};
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum TokenCommand {
    /// Make a token and show it one time. Put it in the OwnTracks app as the password.
    Add(AddArgs),
    /// List the tokens, without their values.
    List,
    /// Revoke a token: the hub refuses it from now on.
    Revoke(RevokeArgs),
}

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// A name for the token, such as the phone that will use it.
    name: String,
    #[command(flatten)]
    to_file: ToFile,
}

#[derive(Debug, clap::Args)]
pub struct RevokeArgs {
    /// The token's ID (see `anima location token list`).
    id: String,
}

impl TokenCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (an API key cannot add a token), or a file that cannot be
    /// written.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Add(args) => args.run(invocation).await,
            Self::List => {
                let hub = invocation.hub()?;
                let answer = hub
                    .run::<LocationTokenList>(location_token_list::Variables)
                    .await?;
                invocation
                    .output
                    .data(&answer.location_ingest_tokens, |tokens| {
                        table(&serde_json::to_value(tokens).unwrap_or_default())
                    })
            }
            Self::Revoke(args) => {
                let hub = invocation.hub()?;
                let variables = location_token_revoke::Variables { id: args.id };
                let answer = hub.run::<LocationTokenRevoke>(variables).await?;
                invocation
                    .output
                    .data(&answer.revoke_location_ingest_token, |token| {
                        table(&json!([token]))
                    })
            }
        }
    }
}

impl AddArgs {
    async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        self.to_file.check()?;
        let hub = invocation.hub()?;
        let variables = location_token_add::Variables { name: self.name };
        let issued = hub
            .run::<LocationTokenAdd>(variables)
            .await?
            .create_location_ingest_token;
        let value = Zeroizing::new(issued.token);
        invocation.output.note(
            "In the OwnTracks app use HTTP mode with the address \
             <hub>/locations/v1/owntracks, any user name, and this token as the password.",
        );
        let token = issued.ingest_token;
        let public = json!({ "ingestToken": {
            "id": token.id, "name": token.name, "createdAt": token.created_at,
        } });
        let secret = Secret {
            label: "location ingest token",
            field: "token",
            value: &value,
            to_file: &self.to_file,
            works: Works::UntilRevoked,
        };
        show_once(invocation, &secret, public)
    }
}

fn table(tokens: &serde_json::Value) -> Table {
    let mut table = Table::new(vec!["NAME", "CREATED", "LAST USED", "REVOKED", "ID"]);
    for token in tokens.as_array().map_or_default(Vec::as_slice) {
        let text = |field: &str| token[field].as_str().unwrap_or("-").to_owned();
        table.row(vec![
            text("name").as_str().into(),
            text("createdAt").as_str().into(),
            text("lastUsedAt").as_str().into(),
            text("revokedAt").as_str().into(),
            text("id").as_str().into(),
        ]);
    }
    table
}
