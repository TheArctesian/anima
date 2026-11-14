//! `anima source connect github`: connects a GitHub account. Without `--from-file` the hub
//! makes the GitHub App and the operator presses Create and Install on GitHub (PRJ-039,
//! PRJ-040, PRJ-041); the command prints each phase. With `--from-file` the App's private key
//! is the secret: it comes from a private file (mode 0600) or from standard input, never from
//! an argument, and is wiped after the request. It is never printed (CMD-025, CMD-014,
//! PRJ-018). The one-time link is shown once, at a terminal, and is in no other output
//! (MCP-004, LOG-007).

pub mod attempt;
pub mod documents;
pub mod flow;
mod lines;
mod open_attempt;
pub mod report;

use std::path::PathBuf;

use zeroize::Zeroize as _;

use self::flow::Request;
use self::report::{Opener, Report, system_browser};
use super::documents::{SourceConnect, source_connect};
use super::render;
use crate::devices::credentials::input::{from_file_only, send_wiping};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ConnectCommand {
    /// Connect a GitHub account: the hub makes the App and you press Create and Install on
    /// GitHub, or give an App's private key with `--from-file`.
    Github(GithubArgs),
}

#[derive(clap::Args)]
pub struct GithubArgs {
    /// A name for the account. Without it, the login of the App's owner.
    #[arg(long, conflicts_with_all = ["state", "cancel"])]
    label: Option<String>,
    /// Make the App in this GitHub organisation, not in your own account.
    #[arg(long, value_name = "LOGIN", conflicts_with_all = ["from_file", "state", "cancel"])]
    organisation: Option<String>,
    /// Print the link and do not open it in a browser.
    #[arg(long, conflicts_with_all = ["from_file", "state", "cancel"])]
    no_browser: bool,
    /// Show the connection in progress, or that none is.
    #[arg(long, conflicts_with_all = ["from_file", "cancel"])]
    state: bool,
    /// Cancel the connection in progress.
    #[arg(long, conflicts_with = "from_file")]
    cancel: bool,
    /// The GitHub App's ID (digits). With `--from-file`.
    #[arg(long, value_parser = digits, requires = "from_file")]
    app_id: Option<String>,
    /// The installation's ID (digits). With `--from-file`.
    #[arg(long, value_parser = digits, requires = "from_file")]
    installation_id: Option<String>,
    /// Read the App's private key (PEM) from this file, or from standard input for `-`. The
    /// file must be mode 0600 and yours. The key is never taken as an argument. It needs
    /// `--label`, `--app-id` and `--installation-id`.
    #[arg(
        long,
        value_name = "PATH",
        requires_all = ["label", "app_id", "installation_id"]
    )]
    from_file: Option<PathBuf>,
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
    /// Connects GitHub and prints the account (or, with `--state` and `--cancel`, the attempt).
    ///
    /// # Errors
    /// [`Failure::Input`] for a key file that is missing, looser than 0600, empty or not text;
    /// the hub's refusal (`CONFLICT` at start while an App waits for Install, `VALIDATION`
    /// when GitHub refuses a key, `UNAVAILABLE` when GitHub cannot be reached, `FORBIDDEN` for
    /// an API key); [`Failure::Ended`] when the attempt failed or was cancelled (exit 1, or 7
    /// for `GITHUB_UNREACHABLE`) and for Ctrl-C (exit 130).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.from_file.is_some() {
            return self.with_key_file(invocation).await;
        }
        let hub = invocation.hub()?;
        if self.state {
            return open_attempt::show(&hub, invocation).await;
        }
        if self.cancel {
            return open_attempt::cancel(&hub, invocation).await;
        }
        self.in_three_presses(&hub, invocation).await
    }

    /// The hub checks the key with GitHub before it stores it.
    async fn with_key_file(self, invocation: &Invocation) -> Result<(), Failure> {
        let key = from_file_only(
            invocation.prompter,
            "The GitHub App private key",
            self.from_file.as_deref(),
        )?;
        let hub = invocation.hub()?;
        let variables = source_connect::Variables {
            input: source_connect::GithubAppInput {
                label: self.label.unwrap_or_default(),
                app_id: self.app_id.unwrap_or_default(),
                installation_id: self.installation_id.unwrap_or_default(),
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

    async fn in_three_presses(self, hub: &Hub, invocation: &Invocation) -> Result<(), Failure> {
        let mut notes = std::io::stderr();
        let mut browser = system_browser;
        let opener: Opener<'_> = &mut browser;
        let mut report = Report {
            notes: &mut notes,
            terminal: invocation.stderr_terminal(),
            json: invocation.output.json(),
            browser: (!self.no_browser).then_some(opener),
        };
        let request = Request {
            label: self.label,
            organisation: self.organisation,
        };
        let last = flow::connect(hub, request, &mut report).await?;
        if invocation.output.json() {
            invocation.output.raw(&last)
        } else {
            invocation.output.data(&last["account"], render::record)
        }
    }
}
