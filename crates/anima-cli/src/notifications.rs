//! `anima notify`: where alerts go (NOT-001..007). Channels (a webhook, ntfy, e-mail or
//! Apprise), the installation's settings, and the record of what was sent. A channel's
//! secrets come from a file or standard input, never an argument (CMD-025).

pub mod channel;
pub mod channel_input;
pub mod channel_list;
pub mod channel_remove;
pub mod channel_test;
pub mod channel_write;
pub mod deliveries;
pub mod documents;
pub mod settings;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum NotifyCommand {
    /// Channels: list, create, update, delete and test.
    Channel {
        #[command(subcommand)]
        command: channel::ChannelCommand,
    },
    /// The certificate warning days and the message template (get, set).
    Settings {
        #[command(subcommand)]
        command: settings::SettingsCommand,
    },
    /// What the hub sent, newest first, and whether it arrived.
    Deliveries(deliveries::DeliveriesArgs),
}

impl NotifyCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown channel, unreadable input or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Channel { command } => command.run(invocation).await,
            Self::Settings { command } => command.run(invocation).await,
            Self::Deliveries(args) => args.run(invocation).await,
        }
    }
}
