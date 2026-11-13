//! `anima source connect github`: connects a GitHub App installation. The App's private key is
//! the secret: it comes from a private file (mode 0600) or from standard input, never from an
//! argument, and is wiped after the request. It is never printed (CMD-025, CMD-014, PRJ-018).

use std::path::PathBuf;

use zeroize::Zeroize as _;

use super::documents::{SourceConnect, source_connect};
use super::render;
use crate::devices::credentials::input::{from_file_only, send_wiping};
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ConnectCommand {
    /// Connect a GitHub App installation, with the App's private key.
    Github(GithubArgs),
}

#[derive(clap::Args)]
pub struct GithubArgs {
    /// A name for the account.
    #[arg(long)]
    label: String,
    /// The GitHub App's ID (digits).
    #[arg(long, value_parser = digits)]
    app_id: String,
    /// The installation's ID (digits).
    #[arg(long, value_parser = digits)]
    installation_id: String,
    /// Read the App's private key (PEM) from this file, or from standard input for `-`. The
    /// file must be mode 0600 and yours. The key is never taken as an argument.
    #[arg(long, value_name = "PATH")]
    from_file: PathBuf,
}

/// Debug output of the arguments names the file, never the key it holds.
impl std::fmt::Debug for GithubArgs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GithubArgs")
            .field("label", &self.label)
            .field("from_file", &self.from_file)
            .finish_non_exhaustive()
    }
}

fn digits(text: &str) -> Result<String, String> {
    if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(text.to_owned())
    } else {
        Err("it must be digits only".to_owned())
    }
}

impl ConnectCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unreadable or exposed key file or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Github(args) => args.run(invocation).await,
        }
    }
}

impl GithubArgs {
    /// Connects the installation and prints the account. The hub checks the key with GitHub
    /// before it stores it.
    ///
    /// # Errors
    /// [`Failure::Input`] for a key file that is missing, looser than 0600, empty or not text;
    /// the hub's refusal (`VALIDATION` when GitHub refuses the key, `UNAVAILABLE` when GitHub
    /// cannot be reached, `FORBIDDEN` for an API key).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let key = from_file_only(
            invocation.prompter,
            "The GitHub App private key",
            Some(&self.from_file),
        )?;
        let hub = invocation.hub()?;
        let variables = source_connect::Variables {
            input: source_connect::GithubAppInput {
                label: self.label,
                app_id: self.app_id,
                installation_id: self.installation_id,
                private_key: key.as_str().to_owned(),
            },
        };
        let answer = send_wiping::<SourceConnect>(&hub, variables, |variables| {
            variables.input.private_key.zeroize();
        })
        .await?;
        invocation
            .output
            .data(&answer.connect_github_app, render::record)
    }
}
