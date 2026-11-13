//! The SSH session a files tunnel needs (K4 Section 9): `openFiles` takes only a live SSH
//! session of the same operator and client address whose terminal tunnel is open. So: start
//! one, open its terminal and keep it alive (its output is dropped), open the files tunnel,
//! do the work, then close both and end the session, whatever the work's outcome.

use std::time::Duration;

use tokio::sync::{mpsc, watch};

use super::exchange::{Files, FilesError};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::HubError;
use crate::invocation::Invocation;
use crate::sessions::start::{end, open_files, start_ssh};
use crate::sessions::terminal::codec::CellSize;
use crate::sessions::terminal::console::pixels;
use crate::sessions::terminal::ending::verdict;
use crate::sessions::terminal::link::{Streams, relay};

/// How long opening the files tunnel may take (the hub opens its own SSH connection).
const FILES_OPENING: Duration = Duration::from_secs(30);

/// Runs `work` on a files tunnel to `device` (ID or exact name).
///
/// # Errors
/// The hub's refusals, a tunnel that fails, or the session ending under the work, as a
/// [`FilesError`] for `describe` to word; `work`'s own failure.
pub async fn with_files<T>(
    invocation: &Invocation,
    device: &str,
    work: impl AsyncFnOnce(&mut Files) -> Result<T, Failure>,
) -> Result<T, Failure> {
    let hub = invocation.hub()?;
    let device_id = resolve(&hub, device).await?;
    let started = start_ssh(&hub, device_id, pixels(CellSize::DEFAULT)).await?;
    if started.wake_sent {
        invocation.output.note(
            "The device was asleep; a wake packet was sent. Opening can take up to two minutes.",
        );
    }
    let session = started.session.clone();
    let wait = started.opening_wait();
    let mut terminal = match hub.open_tunnel(started.ticket, wait).await {
        Ok(socket) => socket,
        Err(error) => {
            let _ended = end(&hub, &session).await;
            return Err(HubError::from(error).into());
        }
    };
    let files_side = async {
        let ticket = open_files(&hub, &session).await?;
        let socket = hub
            .open_tunnel(ticket, FILES_OPENING)
            .await
            .map_err(HubError::from)?;
        let mut files = Files::new(socket);
        let done = work(&mut files).await;
        files.close().await;
        done
    };
    let (keys, sizes, mut sink) = quiet();
    let streams = Streams {
        keys,
        sizes,
        output: &mut sink,
        stop: Box::pin(std::future::pending()),
    };
    let done = tokio::select! {
        done = files_side => done,
        ending = relay(&mut terminal, streams) => {
            let recorded = end(&hub, &session).await;
            Err(verdict(recorded, ending, device)
                .err()
                .unwrap_or_else(|| super::describe(&FilesError::Closed, "")))
        }
    };
    terminal.close().await;
    let _ended = end(&hub, &session).await;
    done
}

/// No input, a fixed 80 by 24 size, and output dropped: the terminal only keeps the session.
fn quiet() -> (
    mpsc::Receiver<Vec<u8>>,
    watch::Receiver<CellSize>,
    std::io::Sink,
) {
    let (_, keys) = mpsc::channel(1);
    let (_, sizes) = watch::channel(CellSize::DEFAULT);
    (keys, sizes, std::io::sink())
}
