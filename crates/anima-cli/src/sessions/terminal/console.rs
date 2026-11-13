//! The operator's terminal (K4 Section 8): raw mode that is always undone, the window size,
//! standard input as a bounded stream, and the signals that end a session. The OS edge of
//! `anima ssh`; `crossterm` does the `unsafe` work, so this crate keeps `unsafe_code` forbidden.

use std::io::{self, Read as _};
use std::pin::Pin;
use std::sync::Once;

use crossterm::terminal;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{mpsc, watch};

use super::codec::CellSize;

/// The pixel range the hub takes for each side of `DisplayInput` (K4 Section 4).
const PIXELS: std::ops::RangeInclusive<i64> = 1..=8192;
/// The cell size assumed when the terminal gives no pixel size.
const CELL_PIXELS: (i64, i64) = (8, 16);
/// Input chunks waiting for the hub, at most.
const QUEUED_INPUT: usize = 16;

/// Raw mode while it lives: no line editing, no echo, keys go to the device as they are.
/// Dropping it restores the terminal, on every return, error and unwinding panic.
pub struct RawMode(());

static PANIC_RESTORES: Once = Once::new();

impl RawMode {
    /// Enters raw mode, and makes a panic restore the terminal before it prints.
    ///
    /// # Errors
    /// The OS error when the terminal cannot change mode.
    pub fn enter() -> io::Result<Self> {
        PANIC_RESTORES.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic| {
                let _restored = terminal::disable_raw_mode();
                previous(panic);
            }));
        });
        terminal::enable_raw_mode()?;
        Ok(Self(()))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _restored = terminal::disable_raw_mode();
    }
}

/// The window size in cells, or 80 by 24 when there is no terminal to ask.
#[must_use]
pub fn size() -> CellSize {
    terminal::size().map_or(CellSize::DEFAULT, |(cols, rows)| {
        CellSize { cols, rows }.clamped()
    })
}

/// The window in pixels for `DisplayInput` (required even for SSH): the terminal's own pixel
/// size when it gives one, else `size` times a common cell.
#[must_use]
pub fn pixels(size: CellSize) -> (i64, i64) {
    let reported = terminal::window_size()
        .ok()
        .filter(|window| window.width > 0 && window.height > 0)
        .map(|window| (i64::from(window.width), i64::from(window.height)));
    let (width, height) = reported.unwrap_or((
        i64::from(size.cols) * CELL_PIXELS.0,
        i64::from(size.rows) * CELL_PIXELS.1,
    ));
    let within = |value: i64| value.clamp(*PIXELS.start(), *PIXELS.end());
    (within(width), within(height))
}

/// Standard input as chunks, read on its own thread so a blocked read never holds the
/// runtime; the channel closes at the end of input. The thread ends with the process.
#[must_use]
pub fn keys() -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel(QUEUED_INPUT);
    let spawned = std::thread::Builder::new()
        .name("anima-stdin".to_owned())
        .spawn(move || {
            let mut stdin = io::stdin().lock();
            let mut buffer = [0_u8; 4096];
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        if sender.blocking_send(buffer[..read].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });
    // With no thread there is no input: the receiver is closed, as at the end of input.
    drop(spawned);
    receiver
}

/// The window size now and after each change (`SIGWINCH`) when `follow`; else only now.
#[must_use]
pub fn sizes(follow: bool) -> watch::Receiver<CellSize> {
    let (sender, receiver) = watch::channel(size());
    if follow && let Ok(mut changes) = signal(SignalKind::window_change()) {
        tokio::spawn(async move {
            while changes.recv().await.is_some() {
                if sender.send(size()).is_err() {
                    break;
                }
            }
        });
    }
    receiver
}

/// Resolves at the first `SIGINT`, `SIGTERM` or `SIGHUP`, so the session ends cleanly and raw
/// mode is undone instead of the process dying in it.
pub fn stop() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let kinds = (
        signal(SignalKind::interrupt()),
        signal(SignalKind::terminate()),
        signal(SignalKind::hangup()),
    );
    Box::pin(async move {
        let (Ok(mut interrupt), Ok(mut terminate), Ok(mut hangup)) = kinds else {
            return std::future::pending().await;
        };
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
            _ = hangup.recv() => {}
        }
    })
}
