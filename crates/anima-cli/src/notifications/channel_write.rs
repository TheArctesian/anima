//! `anima notify channel create` and `update`: the channel's name, settings and secrets come
//! from a JSON document in a file or on standard input (CMD-025).

use std::path::PathBuf;

use serde::Serialize;

use super::channel::resolve;
use super::channel_input::read;
use super::documents::{ChannelCreate, ChannelUpdate, channel_create, channel_update};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The channel as JSON in a file, or - for standard input.
    #[arg(long, value_name = "FILE")]
    from_file: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// The channel's ID or its exact name.
    channel: String,
    /// The channel as JSON in a file, or - for standard input.
    #[arg(long, value_name = "FILE")]
    from_file: PathBuf,
}

impl CreateArgs {
    /// Prints the new channel as a record or, with `--json`, as the K1 answer (no secret).
    ///
    /// # Errors
    /// [`Failure::Input`] for an unreadable document; the hub's refusal (`VALIDATION` names
    /// the fields, `CONFLICT` a name in use).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let input: channel_create::NotificationChannelInput = read(invocation, &self.from_file)?;
        let hub = invocation.hub()?;
        let answer = hub
            .run::<ChannelCreate>(channel_create::Variables { input })
            .await?;
        invocation
            .output
            .data(&answer.create_notification_channel, |channel| {
                saved(
                    &channel.name,
                    &channel.kind,
                    &channel.id,
                    &channel.updated_at,
                )
            })
    }
}

impl UpdateArgs {
    /// Prints the changed channel as a record or, with `--json`, as the K1 answer.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown channel; [`Failure::Input`] for an unreadable
    /// document; the hub's refusal (a changed target needs every secret sent again).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let input: channel_update::NotificationChannelInput = read(invocation, &self.from_file)?;
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.channel).await?;
        let answer = hub
            .run::<ChannelUpdate>(channel_update::Variables { id, input })
            .await?;
        invocation
            .output
            .data(&answer.update_notification_channel, |channel| {
                saved(
                    &channel.name,
                    &channel.kind,
                    &channel.id,
                    &channel.updated_at,
                )
            })
    }
}

fn saved(name: &str, kind: &impl Serialize, id: &str, updated_at: &str) -> Table {
    let mut table = Table::record();
    table.pair("Name", name);
    table.pair("Kind", enum_name(kind));
    table.pair("ID", id);
    table.pair("Updated", updated_at);
    table
}
