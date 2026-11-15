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

/// The most sessions at one time. A connection beyond it is told that the gateway is busy
/// and closed.
pub const MOST_SESSIONS: usize = 64;
/// `GUAC_PROTOCOL_STATUS_SERVER_BUSY`.
const SERVER_BUSY: u16 = 0x0201;
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
    /// The most sessions at one time; [`MOST_SESSIONS`] for the program.
    pub most_sessions: usize,
}

/// Accepts connections until the listener fails for good.
///
/// # Errors
/// Never returns `Ok`; the error is the reason the listener cannot go on.
pub async fn listen(listener: TcpListener, workers: Workers) -> io::Result<()> {
    let free = Arc::new(Semaphore::new(workers.most_sessions));
    loop {
        let connection = match listener.accept().await {
            Ok((connection, _)) => connection,
            Err(error) => {
                // Out of file descriptors, most likely: sessions that end free some.
                eprintln!("anima-gateway: a connection was not accepted: {error}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let Ok(place) = free.clone().try_acquire_owned() else {
            eprintln!("anima-gateway: busy: a connection was refused");
            // Fixed bytes, written without waiting; the listener still reads nothing.
            let busy = guac_protocol::encode(&crate::handshake::error(SERVER_BUSY));
            let _ = connection.try_write(busy.as_bytes());
            continue;
        };
        match start(connection, &workers) {
            Ok(mut worker) => {
                tokio::spawn(async move {
                    let _ = worker.wait().await;
                    drop(place);
                });
            }
            Err(error) => eprintln!("anima-gateway: a worker did not start: {error}"),
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
