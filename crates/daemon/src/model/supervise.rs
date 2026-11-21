//! The servers of the node and their supervision (Section 9, CAG-005, RTC-002, RTC-006): the
//! start with the listener check, the loopback check in each cycle, the waits of a restart,
//! the limit of the temporary folder, and the stop.

use std::collections::BTreeMap;
use std::time::Duration;

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::loopback::{self, Verdict};
use daemon_core::model_work::supervision::{Next, Supervision};
use uuid::Uuid;

use super::server::{self, Launch, Running};
use super::{folders, sockets};

/// The most seconds from a start to a listener on loopback (`start_failed`).
pub const READY_SECONDS: u64 = 120;
/// The most bytes of the temporary folder of a server (`limit_disk`).
pub const TEMPORARY_BYTES: u64 = 2 << 30;

/// The state of a server, as the runtime report spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Ready,
    Restarting,
    Failed(Cause),
    Stopped,
}

impl ServerState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Restarting => "restarting",
            Self::Failed(_) => "failed",
            Self::Stopped => "stopped",
        }
    }
}

/// One server: what starts it, the process, and its record.
pub struct Server {
    pub runtime: &'static str,
    /// Makes the launch of each start with a new port; `None` when no port is free.
    pub launch: Box<dyn Fn() -> Option<(Launch, u16)> + Send>,
    pub running: Option<(Running, u16)>,
    pub record: Supervision,
    pub state: ServerState,
    pub since: u64,
    pub restart_at: Option<u64>,
}

/// The servers of the node.
#[derive(Default)]
pub struct Servers {
    pub map: BTreeMap<Uuid, Server>,
}

/// The verdict over the sockets of each process of the tree, read again now. A tree or a
/// table that does not read counts as exposed.
async fn verdict_of(running: &mut Running, port: u16) -> (Verdict, bool) {
    let table = match running.poll().await {
        Ok(members) => sockets::table(&members).await,
        Err(error) => Err(error),
    };
    match table {
        Ok(table) => (loopback::verdict(&table), loopback::listens(&table, port)),
        Err(_) => (Verdict::Exposed(unreadable()), false),
    }
}

fn unreadable() -> loopback::Socket {
    loopback::Socket {
        protocol: loopback::Protocol::Tcp,
        address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        port: 0,
    }
}

impl Server {
    fn launch_now(&mut self, now: u64) -> Result<(), Cause> {
        let (launch, port) = (self.launch)().ok_or(Cause::StartFailed)?;
        let running = server::spawn(&launch)?;
        self.running = Some((running, port));
        self.state = ServerState::Starting;
        self.since = now;
        self.restart_at = None;
        Ok(())
    }

    async fn end(&mut self, state: ServerState) {
        if let Some((mut running, _)) = self.running.take() {
            server::kill(&mut running).await;
        }
        self.state = state;
    }

    /// One look at a running server: ended, exposed, ready, or too slow. `Some` when it ended.
    async fn look(&mut self, now: u64, temporary_bytes: u64) -> Option<()> {
        let (running, port) = self.running.as_mut()?;
        if let Ok(Some(status)) = running.child.try_wait() {
            let failure = running.launch_failure(status);
            // The main process ended: each other process of its tree ends with it, so no
            // child keeps a listener that the loopback check no longer reads.
            if let Some((mut running, _)) = self.running.take() {
                server::kill(&mut running).await;
            }
            if let Some(step) = failure {
                self.state = ServerState::Failed(Cause::ContainmentFailed(step));
                return None;
            }
            return Some(());
        }
        let (verdict, listens) = verdict_of(running, *port).await;
        if let Verdict::Exposed(_) = verdict {
            self.end(ServerState::Failed(Cause::Listener)).await;
            return None;
        }
        if temporary_bytes > TEMPORARY_BYTES {
            self.end(ServerState::Failed(Cause::LimitDisk)).await;
            return None;
        }
        if listens && self.state != ServerState::Ready {
            self.state = ServerState::Ready;
            self.record.ready(now);
        } else if !listens && now.saturating_sub(self.since) > READY_SECONDS {
            self.end(ServerState::Restarting).await;
            return Some(());
        }
        self.record.tick(now);
        None
    }

    /// One cycle of the supervision at `now` (each 15 seconds, and while a start waits).
    pub async fn cycle(&mut self, now: u64, temporary_bytes: u64) {
        if matches!(self.state, ServerState::Failed(_) | ServerState::Stopped) {
            return;
        }
        if self.look(now, temporary_bytes).await.is_some() {
            self.state = match self.record.ended(now) {
                Next::RestartAt(at) => {
                    self.restart_at = Some(at);
                    ServerState::Restarting
                }
                Next::Failed => ServerState::Failed(Cause::StartFailed),
            };
        }
        if self.restart_at.is_some_and(|at| at <= now) && self.launch_now(now).is_err() {
            self.state = ServerState::Failed(Cause::StartFailed);
        }
    }
}

impl Servers {
    /// `server_start`: starts the server and waits until it listens on loopback, at most
    /// [`READY_SECONDS`] of the `clock`.
    ///
    /// # Errors
    /// `listener`, `start_failed` or `limit_disk`.
    pub async fn start(
        &mut self,
        id: Uuid,
        mut server: Server,
        clock: impl Fn() -> u64,
    ) -> Result<(), Cause> {
        self.stop(id).await;
        server.record.started();
        server.launch_now(clock())?;
        loop {
            let now = clock();
            if server.look(now, 0).await.is_some() {
                server.state = ServerState::Failed(Cause::StartFailed);
            }
            match server.state {
                ServerState::Ready => break,
                ServerState::Failed(cause) => return Err(cause),
                _ if now.saturating_sub(server.since) > READY_SECONDS => {
                    server.end(ServerState::Failed(Cause::StartFailed)).await;
                    return Err(Cause::StartFailed);
                }
                _ => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
        self.map.insert(id, server);
        Ok(())
    }

    /// `server_stop`: `false` when no such server is on this node.
    pub async fn stop(&mut self, id: Uuid) -> bool {
        let Some(mut server) = self.map.remove(&id) else {
            return false;
        };
        if let Some((mut running, _)) = server.running.take() {
            server::stop(&mut running).await;
        }
        true
    }

    /// One cycle over each server; `temporary` gives the folder of a server.
    pub async fn cycle(&mut self, now: u64, temporary: impl Fn(Uuid) -> std::path::PathBuf) {
        for (id, server) in &mut self.map {
            let bytes = folders::bytes_below(&temporary(*id));
            server.cycle(now, bytes).await;
        }
    }

    /// Stops each server: the daemon stops.
    pub async fn stop_all(&mut self) {
        let ids: Vec<Uuid> = self.map.keys().copied().collect();
        for id in ids {
            self.stop(id).await;
        }
    }
}
