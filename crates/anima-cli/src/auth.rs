//! `anima auth`: sign in, sign out and show who is signed in (CMD-010..014). No command here
//! prints a token.

pub mod documents;
pub mod login;
pub mod token_file;
pub mod token_store;

use serde::Serialize;

use crate::failure::Failure;
use crate::hub::{Code, HubError, Token, TokenKind};
use crate::invocation::{Invocation, TokenSource};
use crate::output::{Cell, Table};
use documents::{Logout, Me, logout, me};

#[derive(Debug, clap::Subcommand)]
pub enum AuthCommand {
    /// Sign in to the hub and keep the token in the key store.
    Login {
        /// Read an API key (anima_...) from standard input instead of asking for a password.
        #[arg(long)]
        with_token: bool,
        /// The user name, so it is not asked for.
        #[arg(long)]
        username: Option<String>,
    },
    /// End the stored login token at the hub and delete it here.
    Logout,
    /// Show the hub, the user and the kind of token.
    Status,
}

/// What `auth status` prints; `--json` keeps these names.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    hub: String,
    operator: me::MeMe,
    token_kind: TokenKind,
    token_source: TokenSource,
}

impl AuthCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, a missing value or a store failure.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Login {
                with_token,
                username,
            } => sign_in(invocation, with_token, username).await,
            Self::Logout => sign_out(invocation).await,
            Self::Status => status(invocation).await,
        }
    }
}

async fn sign_in(
    invocation: &Invocation,
    with_token: bool,
    username: Option<String>,
) -> Result<(), Failure> {
    let origin = invocation.origin()?;
    let prompter = invocation.prompter;
    let signed_in = if with_token {
        login::with_token(origin.clone(), prompter, invocation.stdin_terminal()).await?
    } else {
        login::with_password(origin.clone(), prompter, username).await?
    };
    invocation.token_store().write(&origin, &signed_in.token)?;
    let kind = signed_in.token.kind().describe();
    invocation.output.note(&format!(
        "Signed in to {origin} as {} with {kind}.",
        signed_in.username
    ));
    if invocation.has_token_variable() {
        invocation
            .output
            .note("ANIMA_TOKEN is set, and commands use it before the stored token.");
    }
    Ok(())
}

async fn sign_out(invocation: &Invocation) -> Result<(), Failure> {
    let origin = invocation.origin()?;
    let store = invocation.token_store();
    let Some(token) = store.read(&origin)? else {
        invocation
            .output
            .note(&format!("No token is stored for {origin}."));
        return Ok(());
    };
    let kind = token.kind();
    if kind == TokenKind::Login {
        let hub = crate::hub::Hub::new(origin.clone(), Some(token))?;
        match hub.run::<Logout>(logout::Variables).await {
            // An expired or revoked token is already signed out.
            Ok(_)
            | Err(HubError::Refused {
                code: Code::Unauthenticated,
                ..
            }) => {}
            Err(error) => return Err(error.into()),
        }
    }
    store.delete(&origin)?;
    invocation.output.note(&format!("Signed out of {origin}."));
    if kind == TokenKind::ApiKey {
        invocation
            .output
            .note("The API key itself still works until it expires or is revoked.");
    }
    Ok(())
}

async fn status(invocation: &Invocation) -> Result<(), Failure> {
    let (hub, source) = invocation.signed_in()?;
    let answer = hub.run::<Me>(me::Variables).await?;
    let status = Status {
        hub: hub.origin().as_text(),
        operator: answer.me,
        token_kind: hub.token().map_or(TokenKind::Login, Token::kind),
        token_source: source,
    };
    invocation.output.data(&status, |status| {
        let from = match status.token_source {
            TokenSource::Environment => "from ANIMA_TOKEN",
            TokenSource::Store => "from the key store",
        };
        let mut table = Table::record();
        table.pair("Hub", status.hub.as_str());
        table.pair("User", status.operator.username.as_str());
        table.pair("Token", format!("{}, {from}", status.token_kind.describe()));
        table.pair(
            "Authenticator",
            Cell::new(if status.operator.totp_enabled {
                "on"
            } else {
                "off"
            }),
        );
        table
    })
}
