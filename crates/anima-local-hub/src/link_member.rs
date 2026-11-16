//! The link (`anima-link hub`) as the one member that the group starts again alone (F-6).
//!
//! The link needs its relay to start, and with no network it ends. The hub must still serve
//! this computer, so an end of the link never stops the group: the link is started again after
//! a pause that doubles from `first` to `longest`, and the pause is `first` again after the
//! link was up for `steady`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::child::{Child, Spec, Stop, Stopped};
use crate::group::Limits;
use crate::log_file::{LOG_LIMIT, LogFile};
use crate::members::{self, LongSocketPath};
use crate::state::{Link, LinkState};

#[cfg(test)]
mod tests;

/// The pauses between two starts of the link.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    pub first: Duration,
    pub longest: Duration,
    /// A link that was up this long counts as working: the next pause is `first` again.
    pub steady: Duration,
}

/// The pause before the next start, after a pause of `previous` (none before the first) and
/// a run that was up for `up_for`. Pure.
#[must_use]
pub fn pause_after(previous: Option<Duration>, up_for: Duration, backoff: &Backoff) -> Duration {
    match previous {
        Some(previous) if up_for < backoff.steady => {
            previous.saturating_mul(2).min(backoff.longest)
        }
        _ => backoff.first.min(backoff.longest),
    }
}

/// The running keeper of the link. Dropping it kills the link.
#[derive(Debug)]
pub struct LinkMember {
    ask: watch::Sender<bool>,
    keeper: Option<JoinHandle<Vec<String>>>,
    changes: watch::Receiver<LinkState>,
}

/// What the keeper works with.
struct Keeper {
    log: PathBuf,
    limits: Limits,
    clock: fn() -> SystemTime,
    report: watch::Sender<LinkState>,
}

impl LinkMember {
    /// Starts the link of `spec` now, and again after each end until [`LinkMember::stop`].
    /// A `spec` that is an error is written to `log` and the link stays down.
    pub fn start(
        spec: Result<Spec, LongSocketPath>,
        log: PathBuf,
        limits: Limits,
        clock: fn() -> SystemTime,
    ) -> Self {
        let (report, changes) = watch::channel(LinkState::at(Link::Starting, clock()));
        let (ask, stop) = Stop::new();
        let keeper = Keeper {
            log,
            limits,
            clock,
            report,
        };
        Self {
            ask,
            keeper: Some(tokio::spawn(keep(spec, keeper, stop))),
            changes,
        }
    }

    /// The link's state now.
    pub fn state(&mut self) -> LinkState {
        *self.changes.borrow_and_update()
    }

    /// Returns at the next change of the link's state. Cancel-safe.
    pub async fn changed(&mut self) -> LinkState {
        if self.changes.changed().await.is_err() {
            // The keeper is gone, so the state changes no more.
            std::future::pending::<()>().await;
        }
        *self.changes.borrow_and_update()
    }

    /// Stops the link at once, also in a pause or a wait for its ready line. Returns one line
    /// for a link that did not end by itself.
    pub async fn stop(mut self) -> Vec<String> {
        self.ask.send_replace(true);
        match self.keeper.take() {
            Some(keeper) => keeper
                .await
                .unwrap_or_else(|_| vec![format!("the keeper of {} failed", members::LINK)]),
            None => Vec::new(),
        }
    }
}

impl Drop for LinkMember {
    fn drop(&mut self) {
        if let Some(keeper) = self.keeper.take() {
            // The keeper's child is killed when the keeper is dropped.
            keeper.abort();
        }
    }
}

impl Keeper {
    fn tell(&self, link: Link) {
        self.report
            .send_replace(LinkState::at(link, (self.clock)()));
    }

    /// A line of this program in the link's log. A log that cannot be written is no fault.
    fn note(&self, line: &str) {
        if let Ok(mut log) = LogFile::open(&self.log, LOG_LIMIT) {
            let _ = log.line(&format!("anima-local-hub: {line}"));
        }
    }
}

/// How one start of the link ended.
enum Run {
    Ended { up_for: Duration },
    Stopped(Vec<String>),
}

async fn keep(spec: Result<Spec, LongSocketPath>, keeper: Keeper, mut stop: Stop) -> Vec<String> {
    let spec = match spec {
        Ok(spec) => spec,
        Err(refused) => {
            keeper.note(&refused.to_string());
            keeper.tell(Link::Down);
            stop.asked().await;
            return Vec::new();
        }
    };
    let mut pause = None;
    loop {
        let up_for = match run_once(&spec, &keeper, &mut stop).await {
            Run::Ended { up_for } => up_for,
            Run::Stopped(notes) => return notes,
        };
        keeper.tell(Link::Down);
        let next = pause_after(pause, up_for, &keeper.limits.link_backoff);
        pause = Some(next);
        keeper.note(&format!(
            "{} is down; next start in {} s",
            spec.name,
            next.as_secs()
        ));
        tokio::select! {
            biased;
            () = stop.asked() => return Vec::new(),
            () = tokio::time::sleep(next) => {}
        }
        keeper.tell(Link::Starting);
    }
}

/// Starts the link once and keeps it until it ends or a stop is asked for.
async fn run_once(spec: &Spec, keeper: &Keeper, stop: &mut Stop) -> Run {
    let not_up = Run::Ended {
        up_for: Duration::ZERO,
    };
    let limits = &keeper.limits;
    let Ok(log) = LogFile::open(&keeper.log, LOG_LIMIT) else {
        return not_up;
    };
    let (report_end, mut ended) = mpsc::unbounded_channel();
    let mut child = match Child::start(spec, log, report_end) {
        Ok(child) => child,
        Err(error) => {
            keeper.note(&error.to_string());
            return not_up;
        }
    };
    let ready = tokio::select! {
        biased;
        () = stop.asked() => None,
        ready = child.line_where(limits.link_ready, members::link_is_ready) => Some(ready),
    };
    match ready {
        None => return Run::Stopped(stopped(child, limits.link_stop).await),
        Some(Err(error)) => {
            keeper.note(&error.to_string());
            let _ = child.stop(limits.link_stop).await;
            return not_up;
        }
        Some(Ok(())) => {}
    }
    child.stop_reading();
    keeper.tell(Link::Up);
    let up_at = Instant::now();
    tokio::select! {
        biased;
        () = stop.asked() => Run::Stopped(stopped(child, limits.link_stop).await),
        _ = ended.recv() => Run::Ended { up_for: up_at.elapsed() },
    }
}

async fn stopped(child: Child, limit: Duration) -> Vec<String> {
    let name = child.name();
    match child.stop(limit).await {
        Ok(Stopped::Ended(_)) => Vec::new(),
        Ok(Stopped::Killed) => vec![format!(
            "{name} did not stop in {} s and was killed",
            limit.as_secs()
        )],
        Err(error) => vec![error.to_string()],
    }
}
