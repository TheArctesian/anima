//! The gateway's listener: it accepts the hub's connections and starts one worker process
//! for each, with the connection as the worker's standard input. The listener never reads
//! from a connection, so it never holds a credential, and a worker that fails takes one
//! session with it and nothing else. OS work only.

use std::io;
use std::os::fd::{AsFd as _, OwnedFd};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::Semaphore;

/// The most sessions at one time. A connection beyond it is closed at once, and the hub
/// ends that session as it does when the gateway is not there.
pub const MOST_SESSIONS: usize = 64;
/// The argument that makes the program a worker.
pub const WORKER: &str = "session";
/// The option that names another gateway for the protocols this one does not serve.
pub const OTHER_GATEWAY: &str = "--other-gateway";

/// How a worker is started.
#[derive(Clone, Debug)]
pub struct Workers {
    /// The gateway program itself.
    pub program: PathBuf,
    pub other_gateway: Option<String>,
}

/// Accepts connections until the listener fails for good.
///
/// # Errors
/// Never returns `Ok`; the error is the reason the listener cannot go on.
pub async fn listen(listener: TcpListener, workers: Workers) -> io::Result<()> {
    let free = Arc::new(Semaphore::new(MOST_SESSIONS));
    loop {
        let Ok((connection, _)) = listener.accept().await else {
            // Out of file descriptors, most likely: sessions that end free some.
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        let Ok(place) = free.clone().try_acquire_owned() else {
            continue;
        };
        if let Ok(mut worker) = start(connection, &workers) {
            tokio::spawn(async move {
                let _ = worker.wait().await;
                drop(place);
            });
        }
    }
}

/// Starts a worker with `connection` as its standard input. The worker gets no environment
/// and no other open connection.
fn start(connection: TcpStream, workers: &Workers) -> io::Result<Child> {
    let connection = OwnedFd::from(connection.into_std()?);
    let mut command = Command::new(&workers.program);
    command.arg(WORKER);
    if let Some(other) = &workers.other_gateway {
        command.arg(OTHER_GATEWAY).arg(other);
    }
    command
        .env_clear()
        .stdin(Stdio::from(connection))
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
}

/// In a worker: the hub's connection, which the listener gave as standard input.
///
/// # Errors
/// When standard input is not a connection.
pub fn hub_connection() -> io::Result<TcpStream> {
    let connection = std::net::TcpStream::from(io::stdin().as_fd().try_clone_to_owned()?);
    connection.set_nonblocking(true)?;
    let connection = TcpStream::from_std(connection)?;
    connection.set_nodelay(true)?;
    Ok(connection)
}
