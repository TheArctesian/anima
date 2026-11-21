//! The accounts of the programs of this computer, as the programs said them (THR-132): in the
//! memory of the runner only. They are not saved, not logged, not in an update of a thread and
//! not sent to the hub in stage 1b (THR-051). Pure.

use agent_threads::control::AccountRow;
use agent_threads::drive::{AccountFacts, Program};
use chrono::{DateTime, Utc};

/// The newest account of each program.
#[derive(Debug, Default)]
pub struct Accounts {
    rows: Vec<AccountRow>,
}

impl Accounts {
    /// A report of a program at `at`, merged over its earlier reports.
    pub fn note(&mut self, program: Program, facts: AccountFacts, at: DateTime<Utc>) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.program == program) {
            row.facts.merge(facts);
            row.at = at;
        } else {
            self.rows.push(AccountRow { program, facts, at });
        }
    }

    #[must_use]
    pub fn rows(&self) -> Vec<AccountRow> {
        self.rows.clone()
    }
}
