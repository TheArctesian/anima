//! `anima source connect dawarich`: connects a Dawarich server with an API key of its account
//! (R-210). The key is the secret: it is asked for at the terminal with no echo, or comes from
//! a private file (mode 0600) or from standard input, never from an argument, and is wiped
//! after the request. It is never printed (CMD-014, CMD-025).

use std::path::PathBuf;

use zeroize::Zeroize as _;

use super::documents::{SourceConnectDawarich, source_connect_dawarich};
use crate::devices::credentials::input::{password, send_wiping};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::sources::render;

#[derive(clap::Args)]
pub struct DawarichArgs {
    /// A name for the account.
    #[arg(long)]
    label: String,
    /// The address of the Dawarich server: `https`, the host, the port when it is not 443, and
    /// no path.
    #[arg(long, value_name = "ADDRESS")]
    url: String,
    /// Read the API key from this file, or from standard input for `-`. The file must be mode
    /// 0600 and yours. Without it the command asks for the key. The key is never an argument.
    #[arg(long, value_name = "PATH")]
    from_file: Option<PathBuf>,
}

/// Debug output of the arguments names the file, never the key it holds.
impl std::fmt::Debug for DawarichArgs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DawarichArgs")
            .field("label", &self.label)
            .field("url", &self.url)
            .field("from_file", &self.from_file)
            .finish()
    }
}

impl DawarichArgs {
    /// Connects the server and prints the account. The hub reads one point with the key before
    /// it stores it.
    ///
    /// # Errors
    /// [`Failure::Input`] for a key file that is missing, looser than 0600, empty or not text;
    /// the hub's refusal (`VALIDATION` for the address, the key or the label, `UNAVAILABLE`
    /// when the server does not answer, `FORBIDDEN` for an API key of Anima).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let key = password(
            invocation.prompter,
            "The Dawarich API key",
            self.from_file.as_deref(),
        )?;
        let hub = invocation.hub()?;
        let variables = source_connect_dawarich::Variables {
            input: source_connect_dawarich::DawarichInput {
                label: self.label,
                base_url: self.url,
                api_key: key.as_str().to_owned(),
            },
        };
        let answer = send_wiping::<SourceConnectDawarich>(&hub, variables, |variables| {
            variables.input.api_key.zeroize();
        })
        .await?;
        invocation
            .output
            .data(&answer.connect_dawarich, render::record)
    }
}
