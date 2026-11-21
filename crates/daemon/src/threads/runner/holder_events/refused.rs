//! A command of the runner that the holder refused and that no call waits for.

use agent_threads::control::Change;
use agent_threads::drive::DriveEvent;
use uuid::Uuid;

use super::super::Runner;
use crate::threads::wire::{self, Action, Refusal as HolderRefusal};

impl Runner {
    /// The holder refused a command that no call waits for: the log says so, and a line that
    /// did not reach the program is said in its thread.
    pub(in crate::threads::runner) fn not_taken(
        &mut self,
        sent: Option<wire::Command>,
        why: HolderRefusal,
    ) {
        let Some(wire::Command {
            action: Action::Write { thread, .. },
            ..
        }) = sent
        else {
            tracing::warn!(?why, "the holder refused a command of the runner");
            return;
        };
        let Ok(id) = Uuid::parse_str(&thread) else {
            return;
        };
        tracing::warn!(%id, ?why, "the holder did not write a line to the program");
        self.publish(
            id,
            Change::Said {
                event: DriveEvent::Notice {
                    text: "A line did not reach the program: the holder did not take it."
                        .to_owned(),
                },
            },
        );
    }
}
