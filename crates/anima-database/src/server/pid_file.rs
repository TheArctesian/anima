//! What `postmaster.pid` says about the server that wrote it. Pure.
//!
//! PostgreSQL writes one value on each line: 1 the process ID, 3 the start time in seconds
//! since 1970, and 8 the status, which is `ready` when the server accepts connections.
//! `pg_ctl` reads the same lines to decide that a start is complete.

use std::time::SystemTime;

use super::epoch_seconds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PidFile {
    process_id: u32,
    started_at: u64,
    is_ready: bool,
}

impl PidFile {
    /// `None` while the file is not yet whole.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let process_id = lines.next()?.trim().parse().ok()?;
        let started_at = lines.nth(1)?.trim().parse().ok()?;
        let status = lines.nth(4)?;
        Some(Self {
            process_id,
            started_at,
            is_ready: status.trim() == "ready",
        })
    }

    /// Whether this file is the ready mark of the child `process_id` that was started at
    /// `spawned_at`. A file from before the start is stale, also when an old server had the
    /// number that the new child has now.
    pub fn is_ready_for(self, process_id: u32, spawned_at: SystemTime) -> bool {
        self.is_ready
            && self.process_id == process_id
            && self.started_at >= epoch_seconds(spawned_at)
    }
}
