//! `anima web suggestion accept|dismiss`: the hub suggests a web interface when an HTTP check
//! passes (WEB-002); the operator takes it or turns it down. The argument is the check's ID,
//! which `anima web list` prints beside each suggestion.

use super::documents::{
    WebSuggestionAccept, WebSuggestionDismiss, web_suggestion_accept, web_suggestion_dismiss,
};
use super::shown::Shown;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Subcommand)]
pub enum SuggestionCommand {
    /// Make the suggestion a web interface.
    Accept(CheckArgs),
    /// Stop suggesting it.
    Dismiss(CheckArgs),
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// The ID of the check the suggestion came from.
    check: String,
}

impl SuggestionCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// The hub's refusal, such as `NOT_FOUND` when there is no such suggestion.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        match self {
            Self::Accept(args) => {
                let variables = web_suggestion_accept::Variables {
                    check_id: args.check,
                };
                let web = hub
                    .run::<WebSuggestionAccept>(variables)
                    .await?
                    .accept_web_interface_suggestion;
                invocation.output.data(&web, |web| {
                    Shown {
                        id: &web.id,
                        label: &web.label,
                        url: &web.url,
                        open_in: &enum_name(&web.open_in),
                        pinned_fingerprint: web.pinned_fingerprint_sha256.as_deref(),
                    }
                    .table()
                })
            }
            Self::Dismiss(args) => {
                let variables = web_suggestion_dismiss::Variables {
                    check_id: args.check,
                };
                let answer = hub.run::<WebSuggestionDismiss>(variables).await?;
                invocation.output.data(&answer, |answer| {
                    let mut table = Table::record();
                    table.pair(
                        "Dismissed",
                        answer.dismiss_web_interface_suggestion.as_str(),
                    );
                    table
                })
            }
        }
    }
}
