//! Buffered shutdown requests, installed before database startup begins.
#[cfg(windows)]
pub use anima_windows::shutdown::Requested;
#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind, signal};

#[cfg(unix)]
pub struct Requested {
    terminate: Signal,
    interrupt: Signal,
}

#[cfg(unix)]
impl Requested {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            terminate: signal(SignalKind::terminate())?,
            interrupt: signal(SignalKind::interrupt())?,
        })
    }

    pub async fn wait(&mut self) {
        tokio::select! {
            _ = self.terminate.recv() => {},
            _ = self.interrupt.recv() => {},
        }
    }
}
