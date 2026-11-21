//! The local socket of the runner (TH12, THR-128, THR-045): a socket file of mode 0600 in the
//! folder of the threads (0700). Only a peer of this user is served; it cannot tell the
//! `anima` command from a different program of the same user (OD-THR18).

use std::io;
use std::path::Path;

use agent_threads::control::{Call, MAX_FRAME, Reply, SOCKET};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// A call of a client, with the way back to that client.
#[derive(Debug)]
pub struct Incoming {
    pub call: Call,
    pub client: mpsc::UnboundedSender<Reply>,
}

/// Binds the socket of the runner in `dir`, in place of one an older runner left.
///
/// # Errors
/// The I/O error of the socket file.
pub fn bind(dir: &Path) -> io::Result<UnixListener> {
    use std::os::unix::fs::PermissionsExt as _;
    let socket = dir.join(SOCKET);
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Accepts clients and gives their calls to the runner, until the runner is gone.
pub async fn accept(listener: UnixListener, calls: mpsc::Sender<Incoming>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(serve(stream, calls.clone()));
            }
            Err(error) => {
                tracing::warn!(%error, "the runner could not accept a client");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        if calls.is_closed() {
            return;
        }
    }
}

/// One client: each line is a call; a line that is too long or does not read ends the client.
async fn serve(stream: UnixStream, calls: mpsc::Sender<Incoming>) {
    let same_user = stream
        .peer_cred()
        .is_ok_and(|peer| peer.uid() == rustix::process::getuid().as_raw());
    if !same_user {
        return;
    }
    let (reading, mut writing) = stream.into_split();
    let (client, mut replies) = mpsc::unbounded_channel::<Reply>();
    let writer = tokio::spawn(async move {
        while let Some(reply) = replies.recv().await {
            let mut text = serde_json::to_string(&reply).unwrap_or_default();
            text.push('\n');
            if writing.write_all(text.as_bytes()).await.is_err() {
                break;
            }
        }
    });
    let limit = u64::try_from(MAX_FRAME)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut reader = BufReader::new(reading).take(limit);
    let mut line = Vec::new();
    loop {
        line.clear();
        reader.set_limit(limit);
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) if !line.ends_with(b"\n") => break,
            Ok(_) => {}
        }
        let Ok(call) = serde_json::from_slice::<Call>(line.trim_ascii_end()) else {
            break;
        };
        let incoming = Incoming {
            call,
            client: client.clone(),
        };
        if calls.send(incoming).await.is_err() {
            break;
        }
    }
    drop(client);
    // The runner may still hold the way back of a watch; the writer ends when it lets go.
    let _ = writer.await;
}
