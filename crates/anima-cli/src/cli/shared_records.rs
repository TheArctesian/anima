//! The nouns for what every extension shares: the timeline and the links between records.

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{places, record_links, timeline};

#[derive(Debug, clap::Subcommand)]
pub enum SharedRecordCommand {
    /// What happened, from the records of every domain (list).
    Timeline {
        #[command(subcommand)]
        command: timeline::TimelineCommand,
    },
    /// Links between records of different extensions (list, add, remove).
    RecordLink {
        #[command(subcommand)]
        command: record_links::RecordLinkCommand,
    },
    /// Places: where things are (list, show, add, set, merge, split, remove, site).
    Place {
        #[command(subcommand)]
        command: places::PlaceCommand,
    },
}

impl SharedRecordCommand {
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Timeline { command } => command.run(invocation).await,
            Self::RecordLink { command } => command.run(invocation).await,
            Self::Place { command } => command.run(invocation).await,
        }
    }
}
