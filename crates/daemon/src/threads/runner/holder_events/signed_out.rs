//! The end of a turn of Claude Code with "Not logged in" (TH99): words that say whether the
//! person must sign in or only send the prompt again.

use agent_threads::control::Change;
use agent_threads::drive::DriveEvent;
use uuid::Uuid;

use super::super::Runner;
use super::super::login::sign_in;
use super::super::thread::Step;

/// A step that ends a turn of Claude Code with the fault of its sign-in.
pub(super) fn not_logged_in(step: &Step) -> bool {
    matches!(
        step,
        Step::Change(Change::Said {
            event: DriveEvent::TurnEnded { detail: Some(detail), .. },
        }) if detail == "authentication_failed"
    )
}

impl Runner {
    /// Adds to the thread the words for its "Not logged in", after a look at the Keychain.
    pub(super) async fn tell_sign_in(&mut self, id: Uuid) {
        let text = sign_in::words(sign_in::of_claude(&self.env).await).to_owned();
        self.publish(
            id,
            Change::Said {
                event: DriveEvent::Notice { text },
            },
        );
    }
}
