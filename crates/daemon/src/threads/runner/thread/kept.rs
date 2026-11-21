//! The updates of a thread that the runner keeps for a client that starts to watch, with a
//! bound of count and of bytes (THR-045). Pure.

use agent_threads::control::{Change, Update};

use super::{KEPT, KEPT_BYTES, Thread};

impl Thread {
    /// The updates the runner keeps, for a client that starts to watch.
    pub fn kept(&self) -> impl Iterator<Item = &Update> {
        self.kept.iter().map(|(update, _)| update)
    }

    /// The newest updates the runner keeps, at most `most` of them and `bytes` of them, oldest
    /// first: what a new watch of a client gets of this thread.
    pub fn kept_newest(&self, most: usize, bytes: usize) -> impl Iterator<Item = &Update> {
        let mut used = 0;
        let mut from = self.kept.len();
        for (at, (_, size)) in self.kept.iter().enumerate().rev() {
            if self.kept.len() - at > most || used + size > bytes {
                break;
            }
            used += size;
            from = at;
        }
        self.kept.range(from..).map(|(update, _)| update)
    }

    /// Gives `change` the next number of the thread and keeps it.
    pub fn note(&mut self, change: Change) -> Update {
        self.seq += 1;
        let update = Update {
            thread: self.row.id,
            seq: self.seq,
            change,
        };
        let bytes = serde_json::to_vec(&update).map_or(0, |json| json.len());
        self.kept.push_back((update.clone(), bytes));
        self.kept_bytes += bytes;
        while self.kept.len() > KEPT || self.kept_bytes > KEPT_BYTES {
            let Some((_, gone)) = self.kept.pop_front() else {
                break;
            };
            self.kept_bytes -= gone;
        }
        update
    }
}
