//! The running files tunnel (K4 Sections 8.4, 8.5 and 9): one `select!` over the SSH session's
//! end, the client socket, the one device request in flight and four timers, until the first
//! cause. Each branch is cancel safe; sends to the client happen in branch bodies under a time
//! limit that races the session's end; no lock is held.
//!
//! Backpressure: a download reads the next chunk only after the previous frame was sent; an
//! upload reads the next client frame only after the device took the previous one. While a
//! request other than a download's read waits on the device, the client is not read and its
//! silence does not count; a request that waits past the device wait fails the tunnel.
//!
//! How the tunnel closes (no `end` message, K4 Section 9 has none): the client leaving, 1000;
//! the SSH session ending, 1000; hub shutdown, 1001; a client message outside Section 9 or
//! over a limit, 1002; the device, the link or the hub's stores failing, 1011; client silence
//! or a send that stalls, 4002.

use std::{pin::Pin, sync::Arc, time::Duration};

use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use futures_util::FutureExt as _;
use tokio::{
    sync::watch,
    time::{Instant, sleep_until, timeout},
};

use super::{
    device::{Asked, Broken, Handle, Remote},
    protocol::{self, Listing},
    route::Timing,
    trail::Trail,
    transfer::{Direction, Outcome, Transfer},
};
use crate::sessions::{
    EndCause, EndReason,
    end::{CLOSE_WAIT, Cause},
    upgrade::{Quiet, close},
};

/// One request to the device and what it answered.
pub(super) enum Done {
    Listed(Asked<Listing>),
    /// A download's file opened, with its size.
    Readable(Asked<(Handle, u64)>),
    /// An upload's file created.
    Created(Asked<Handle>),
    /// A download's read and how much it asked for.
    Read(Result<Vec<u8>, Broken>, u32),
    Wrote(Result<(), Broken>),
    Closed(Result<(), Broken>),
    Alive(Result<(), Broken>),
}

/// The one device request in flight.
pub(super) type Work = Pin<Box<dyn Future<Output = Done> + Send>>;

/// Where the transfer in flight stands.
#[derive(Debug)]
pub(super) enum Stage {
    /// Its flags check and open are in flight.
    Opening,
    /// The file is open: a download reads; an upload waits for frames or writes one.
    Open(Handle),
    /// The upload was refused: the client's frames, up to its size, are dropped.
    Discarding,
    /// The file is closing (an unfinished upload is also removed); then the transfer ends so.
    Closing(Outcome),
}

/// The transfer in flight, at most one (K4 Section 9).
#[derive(Debug)]
pub(super) struct Active {
    pub(super) transfer: Transfer,
    pub(super) stage: Stage,
}

/// The SSH session's end signal: resolves once with the first cause, or when the session's
/// tunnel deregisters, then lets the signal go, so `endSession` does not wait on this tunnel.
pub(super) struct SessionEnd {
    signal: Option<watch::Receiver<Option<EndCause>>>,
    reason: Option<EndReason>,
}

impl SessionEnd {
    pub(super) const fn new(signal: watch::Receiver<Option<EndCause>>) -> Self {
        Self {
            signal: Some(signal),
            reason: None,
        }
    }

    /// The reason the session ended. Cancel safe.
    pub(super) async fn ended(&mut self) -> EndReason {
        if let Some(reason) = self.reason {
            return reason;
        }
        let Some(signal) = self.signal.as_mut() else {
            return std::future::pending().await;
        };
        let reason = loop {
            if let Some(cause) = signal.borrow_and_update().clone() {
                break cause.reason;
            }
            if signal.changed().await.is_err() {
                break EndReason::RemoteClosed;
            }
        };
        self.signal = None;
        self.reason = Some(reason);
        reason
    }
}

/// One step of the loop: whichever came first.
enum Turn {
    Ended(EndReason),
    Client(Option<Result<Message, axum::Error>>),
    Done(Done),
    HubSilent,
    ClientIdle,
    WorkStalled,
    DeviceQuiet,
}

/// One upgraded files tunnel.
pub(super) struct Relay {
    pub(super) client: WebSocket,
    pub(super) remote: Arc<Remote>,
    pub(super) session: SessionEnd,
    pub(super) trail: Trail,
    pub(super) timing: Timing,
    /// Most entries one `entries` message holds.
    pub(super) max_entries: usize,
    pub(super) work: Option<Work>,
    pub(super) active: Option<Active>,
    /// When the hub's silence earns the client a `ping`.
    pub(super) hub_quiet: Quiet,
    /// When the client's silence ends the tunnel.
    pub(super) client_quiet: Quiet,
    /// When the request in flight has waited on the device too long.
    pub(super) work_due: Instant,
    /// When an idle link gets a request, so it does not end for want of traffic.
    pub(super) device_quiet: Quiet,
}

/// From `now`, the instant `wait` later.
pub(super) fn after(wait: Duration) -> Instant {
    Instant::now() + wait
}

impl Relay {
    /// Relays until the first cause, then ends any transfer in flight and closes the client.
    pub(super) async fn run(mut self) {
        let cause = loop {
            let turn = self.next_turn().await;
            if let Err(cause) = self.on_turn(turn).await {
                break cause;
            }
        };
        let work = self.work.take();
        if let Some(active) = self.active.take() {
            let outcome = if cause == Cause::GatewayError {
                Outcome::Failed
            } else {
                Outcome::Cancelled
            };
            self.abandon(active, work, outcome).await;
        }
        close(self.client, None, cause).await;
    }

    /// Whether the client is read now: when no request is in flight, or beside a download's
    /// read, so `cancel` and `ping` come through while frames go out.
    fn reads_client(&self) -> bool {
        self.work.is_none()
            || matches!(
                &self.active,
                Some(Active {
                    transfer: Transfer {
                        direction: Direction::Download,
                        ..
                    },
                    stage: Stage::Open(_),
                })
            )
    }

    async fn next_turn(&mut self) -> Turn {
        let reading = self.reads_client();
        let working = self.work.is_some();
        let idle = !working && self.active.is_none();
        tokio::select! {
            reason = self.session.ended() => Turn::Ended(reason),
            message = self.client.recv(), if reading => Turn::Client(message),
            done = finished(&mut self.work), if working => Turn::Done(done),
            () = sleep_until(self.hub_quiet.due()) => Turn::HubSilent,
            () = sleep_until(self.client_quiet.due()), if reading => Turn::ClientIdle,
            () = sleep_until(self.work_due), if working => Turn::WorkStalled,
            () = sleep_until(self.device_quiet.due()), if idle => Turn::DeviceQuiet,
        }
    }

    async fn on_turn(&mut self, turn: Turn) -> Result<(), Cause> {
        match turn {
            Turn::Ended(EndReason::HubShutdown) => Err(Cause::HubShutdown),
            Turn::Ended(_) => Err(Cause::RemoteClosed),
            Turn::Client(message) => self.on_client(message),
            Turn::Done(done) => self.on_done(done).await,
            Turn::HubSilent => self.send_text(protocol::PING).await,
            Turn::ClientIdle => match self.client.recv().now_or_never() {
                Some(message) => self.on_client(message),
                None => Err(Cause::Timeout),
            },
            Turn::WorkStalled => Err(self.broken(Broken::Sftp)),
            Turn::DeviceQuiet => {
                let remote = Arc::clone(&self.remote);
                self.start(async move { Done::Alive(remote.alive().await) });
                Ok(())
            }
        }
    }

    /// Starts `work` as the one request in flight, due within the device wait.
    pub(super) fn start(&mut self, work: impl Future<Output = Done> + Send + 'static) {
        self.work = Some(Box::pin(work));
        self.work_due = after(self.timing.device);
    }

    /// Logs a failure with the device as the tunnel's end: no path, host or device text.
    pub(super) fn broken(&self, broken: Broken) -> Cause {
        let session = self.trail.ticket.session;
        tracing::warn!(%session, error = %broken, "the files tunnel failed");
        Cause::GatewayError
    }

    /// Sends a hub text message.
    pub(super) async fn send_text(&mut self, text: &'static str) -> Result<(), Cause> {
        self.send(Message::Text(Utf8Bytes::from_static(text))).await
    }

    /// Sends one message, unless the session ends first. A failed send is the client
    /// leaving; one that stalls past the client wait is a timeout (K4 Section 8.5).
    pub(super) async fn send(&mut self, message: Message) -> Result<(), Cause> {
        let wait = self.timing.client_idle;
        tokio::select! {
            reason = self.session.ended() => Err(if reason == EndReason::HubShutdown {
                Cause::HubShutdown
            } else {
                Cause::RemoteClosed
            }),
            sent = timeout(wait, self.client.send(message)) => match sent {
                Ok(Ok(())) => {
                    self.hub_quiet.restart(Instant::now());
                    Ok(())
                }
                Ok(Err(_)) => Err(Cause::OperatorClosed),
                Err(_) => Err(Cause::Timeout),
            },
        }
    }

    /// Ends a transfer the tunnel's end cut short: an upload the hub created, or whose
    /// exclusive create is still in flight in `work` and succeeds, is closed and removed, best
    /// effort within [`CLOSE_WAIT`]; then the audit entry is written.
    async fn abandon(&mut self, active: Active, work: Option<Work>, outcome: Outcome) {
        let Active { transfer, stage } = active;
        let audited = !matches!(stage, Stage::Discarding);
        if transfer.direction == Direction::Upload {
            let remote = Arc::clone(&self.remote);
            let _left = timeout(CLOSE_WAIT, leave_upload(&remote, stage, work, &transfer)).await;
        }
        if audited {
            self.audit(&transfer, outcome, None).await;
        }
    }

    /// Writes the transfer's audit entry and event; a failure is logged and returned.
    pub(super) async fn audit(
        &mut self,
        transfer: &Transfer,
        outcome: Outcome,
        reason: Option<protocol::Refusal>,
    ) -> bool {
        match self.trail.record(transfer, outcome, reason).await {
            Ok(()) => true,
            Err(error) => {
                tracing::error!(%error, "the files tunnel could not record a transfer");
                false
            }
        }
    }
}

/// Closes and removes an unfinished upload at `stage`. While it is opening, its create in
/// `work` is awaited: only a create that succeeded made a file, and being exclusive, that
/// file is the hub's; a refused or failed create leaves what is there alone.
async fn leave_upload(remote: &Remote, stage: Stage, work: Option<Work>, transfer: &Transfer) {
    let handle = match stage {
        Stage::Open(handle) => Some(handle),
        Stage::Opening => match work {
            Some(work) => match work.await {
                Done::Created(Ok(Ok(handle))) => Some(handle),
                _ => return,
            },
            None => return,
        },
        Stage::Closing(_) => None,
        Stage::Discarding => return,
    };
    if let Some(handle) = &handle {
        let _closed = remote.close(handle).await;
    }
    let _removed = remote.remove(&transfer.path).await;
}

/// The request in flight, or never when there is none.
async fn finished(work: &mut Option<Work>) -> Done {
    match work {
        Some(work) => work.as_mut().await,
        None => std::future::pending().await,
    }
}
