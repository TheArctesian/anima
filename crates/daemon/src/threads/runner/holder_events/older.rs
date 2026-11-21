//! A holder of an older build or protocol than the runner (TH98, SF-235): its
//! words for the person, and its end when no program of a thread runs.

use super::super::Runner;
use crate::threads::wire::Action;

/// What the runner knows of the holder from its hello and from the Quit it sent (THR-113, TH98).
#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four facts of the holder that change at different times"
)]
pub(in crate::threads::runner) struct Standing {
    /// The holder speaks the version of this runner; until it says so, nothing starts.
    pub fits: bool,
    /// The holder is of an older build or protocol than this runner.
    pub older: bool,
    /// The holder speaks a different protocol but is of the file of this runner: the runner is
    /// the older one.
    pub newer: bool,
    /// A Quit went to the holder and it did not refuse it: nothing starts until the next holder
    /// says hello, so that no program starts in a holder that ends.
    pub quitting: bool,
    /// When the runner last asked the older holder to end.
    pub quit: Option<std::time::Instant>,
    /// The command of the Quit that waits for its answer.
    pub quit_command: Option<u64>,
}

impl Runner {
    /// Asks an older holder to end when no program of a thread runs, so that the daemon starts
    /// the holder of its own build on the next connection (TH98). The holder refuses while a
    /// program runs; the runner asks again a minute later, also after a new connection, so that
    /// nothing can make a loop of quick ends and starts.
    pub(in crate::threads::runner) async fn replace_older_holder(&mut self) {
        let asked = self
            .holder
            .quit
            .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(60));
        if self.holder.older && !asked && self.running() == 0 {
            self.holder.quit = Some(std::time::Instant::now());
            self.holder.quitting = true;
            self.holder.quit_command = self.command(Action::Quit).await;
            // A Quit that did not reach the holder ends nothing: starts go on.
            self.holder.quitting = self.holder.quit_command.is_some();
        }
    }

    /// The answer to the Quit `command`, if it is that; a refusal means that a program runs, so
    /// the holder stays and starts go on. True when the answer was the Quit's.
    pub(in crate::threads::runner) fn quit_answered(
        &mut self,
        command: u64,
        outcome: &crate::threads::wire::Outcome,
    ) -> bool {
        if self.holder.quit_command != Some(command) {
            return false;
        }
        self.holder.quit_command = None;
        if matches!(outcome, crate::threads::wire::Outcome::Refused { .. }) {
            self.holder.quitting = false;
        }
        true
    }

    /// The words of a holder that is not of the build of this runner, while it runs (TH98).
    pub(in crate::threads::runner) fn holder_words(&self) -> Option<String> {
        if self.holder.quitting {
            return Some(QUITTING.to_owned());
        }
        if self.holder.newer {
            return Some(NEWER.to_owned());
        }
        self.holder
            .older
            .then(|| older_holder(self.holder.fits, self.running()))
    }
}

/// The words while an older holder ends and the holder of this build starts.
const QUITTING: &str = "The holder of the threads is being replaced by the one of this build. Start the thread again in a few seconds.";

/// The words when the daemon is older than the holder: an update replaced the file, and the
/// daemon that runs is still the one before it.
const NEWER: &str = "The daemon is older than the holder of the threads: an update replaced it and it did not start again yet, so no thread starts. Quit and open Anima, or restart the computer.";

/// The words of an older holder: what it means and when it is replaced (TH98).
#[must_use]
pub fn older_holder(speaks: bool, running: u32) -> String {
    let what = if speaks {
        "The holder of the threads is older than the daemon, so the corrections of this build are not in force for its programs."
    } else {
        "The holder of the threads is older than the daemon and speaks a different protocol, so it takes no new thread."
    };
    format!(
        "{what} It is replaced by itself when no thread runs ({running} running now); stop them to replace it at once."
    )
}
