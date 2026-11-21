//! The end of a turn of Claude Code with "Not logged in" (TH99): words that say whether the
//! person must sign in or only send the prompt again.

use agent_threads::control::Change;
use agent_threads::drive::{DriveEvent, Program};
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
    /// Adds to a thread of Claude Code the words for its "Not logged in", after a look at the
    /// Keychain; at most once a minute for one thread, so that a program that says it again and
    /// again does not hold the loop of the runner.
    pub(super) async fn tell_sign_in(&mut self, id: Uuid) {
        let claude = self
            .threads
            .get(&id)
            .is_some_and(|thread| thread.program() == Program::ClaudeCode);
        let recent = self
            .sign_in_told
            .get(&id)
            .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(60));
        if !claude || recent {
            return;
        }
        self.sign_in_told
            .retain(|thread, _| self.threads.contains_key(thread));
        self.sign_in_told.insert(id, std::time::Instant::now());
        let text = sign_in::words(sign_in::of_claude(&self.env).await).to_owned();
        self.publish(
            id,
            Change::Said {
                event: DriveEvent::Notice { text },
            },
        );
    }
}
