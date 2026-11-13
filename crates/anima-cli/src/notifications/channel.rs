//! `anima notify channel`: the verbs, and how a channel is named on the command line.

use super::channel_list::ListArgs;
use super::channel_remove::DeleteArgs;
use super::channel_test::TestArgs;
use super::channel_write::{CreateArgs, UpdateArgs};
use super::documents::{ChannelList, channel_list};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;

/// The shape of the document `create` and `update` read, for `--help`. It is the K1
/// `NotificationChannelInput` as JSON; exactly one key under `settings`.
const DOCUMENT_HELP: &str = r#"The document is JSON in the shape of K1 NotificationChannelInput; exactly one key under "settings":
  {"name": "ops", "onByDefault": true,
   "settings": {"ntfy": {"server": "https://ntfy.sh", "topic": "my-topic", "token": "tk_..."}}}
Other settings: "webhook" {"url", "method", "headers": [{"name", "value"}]}, "email" {"host", "port",
"security", "username", "password", "from", "to": []}, "apprise" {"server", "mode", "urls": [], "key", "tags": []}.
On update, a secret left out keeps its stored value. Pass the file with --from-file, or - for standard input."#;

#[derive(Debug, clap::Subcommand)]
pub enum ChannelCommand {
    /// List channels. Secrets are never shown: only whether one is stored.
    List(ListArgs),
    /// Make a channel from a JSON document.
    #[command(after_help = DOCUMENT_HELP)]
    Create(CreateArgs),
    /// Replace a channel's settings from a JSON document.
    #[command(after_help = DOCUMENT_HELP)]
    Update(UpdateArgs),
    /// Delete a channel by ID or exact name.
    Delete(DeleteArgs),
    /// Send a test message through a channel and show the result.
    Test(TestArgs),
}

impl ChannelCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown channel, unreadable input or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Update(args) => args.run(invocation).await,
            Self::Delete(args) => args.run(invocation).await,
            Self::Test(args) => args.run(invocation).await,
        }
    }
}

/// The ID `channel` names: an ID as it is, or the one channel with exactly that name.
///
/// # Errors
/// [`Failure::NotFound`] when no channel has the name; the hub's refusal.
pub async fn resolve(hub: &Hub, channel: &str) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(channel).is_ok() {
        return Ok(channel.to_owned());
    }
    let channels = hub
        .run::<ChannelList>(channel_list::Variables)
        .await?
        .notification_channels;
    channels
        .into_iter()
        .find(|candidate| candidate.name == channel)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("There is no notification channel named {channel}."),
            hint: Some(Hint::run("anima notify channel list")),
        })
}
