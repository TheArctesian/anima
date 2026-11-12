//! The one install that may run at a time, and its stop when the app exits (MET-019): as the
//! builder's steps, no agent run outlives the app. No lock: two flags, and the claim is owned by
//! the blocking work, so it is freed when the work ends even if no `await` resumes.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use super::error::AgentError;

/// How long the app waits on exit for a stopped agent to be killed and reaped.
const STOP_PATIENCE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(10);

#[derive(Debug, Default)]
pub struct Installing {
    busy: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

/// Holds [`Installing`] for one install; dropping it frees it.
#[derive(Debug)]
pub struct Claim {
    busy: Arc<AtomicBool>,
    /// The flag the agent runs watch.
    pub stop: Arc<AtomicBool>,
}

impl Installing {
    /// Claims the one install.
    ///
    /// # Errors
    /// [`AgentError::Busy`] while another install holds it.
    pub fn claim(&self) -> Result<Claim, AgentError> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Claim {
                busy: Arc::clone(&self.busy),
                stop: Arc::clone(&self.stop),
            })
            .map_err(|_| AgentError::Busy)
    }

    /// Stops a running install and waits a little for its agent to be killed and reaped. Called
    /// once, on exit; no install starts after it.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        let end = Instant::now() + STOP_PATIENCE;
        while self.busy.load(Ordering::Acquire) && Instant::now() < end {
            thread::sleep(POLL);
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}
