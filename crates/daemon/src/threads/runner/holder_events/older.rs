//! A holder of an older build or protocol than the runner (TH98, SF-NEW-holderupdate): its
//! words for the person, and its end when no program of a thread runs.

use super::super::Runner;
use crate::threads::wire::Action;

impl Runner {
    /// Asks an older holder to end when no program of a thread runs, so that the daemon starts
    /// the holder of its own build on the next connection (TH98). The holder refuses while a
    /// program runs; the runner asks again a minute later, also after a new connection, so that
    /// nothing can make a loop of quick ends and starts.
    pub(in crate::threads::runner) async fn replace_older_holder(&mut self) {
        let asked = self
            .holder_quit
            .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(60));
        if self.holder_older && !asked && self.running() == 0 {
            self.holder_quit = Some(std::time::Instant::now());
            self.command(Action::Quit).await;
        }
    }

    /// The words of a holder that is not of the build of this runner, while it runs (TH98).
    pub(in crate::threads::runner) fn holder_words(&self) -> Option<String> {
        if self.holder_newer {
            return Some(NEWER.to_owned());
        }
        self.holder_older
            .then(|| older_holder(self.holder_fits, self.running()))
    }
}

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
