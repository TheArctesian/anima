//! Questions with the terminal's echo off from before the first one is shown until after the
//! last answer (CMD-010). Keys typed ahead are never echoed by the terminal: they wait in its
//! input queue and become the next answer. An answer that may be seen is shown by this program
//! as it is typed; a secret shows nothing at all.

use std::fs::File;
use std::io::{self, Read as _, Write};
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::signal::unix::{SignalKind, signal};
use zeroize::Zeroizing;

use super::line::{Ending, Key, Keys, Line};
use super::{MAX_ANSWER, prompt_text, terminal_failure};
use crate::failure::Failure;
use crate::raw_mode::RawMode;

/// Whether a hidden question is on screen, so that a signal ends its line before the exit.
static ON_SCREEN: AtomicBool = AtomicBool::new(false);
static SIGNALS: Once = Once::new();

/// The terminal with echo off while it lives; dropping it restores the previous mode.
pub struct Hidden {
    keys: Keys,
    raw: Option<RawMode>,
}

impl Hidden {
    /// Turns echo off; nothing has been shown yet.
    ///
    /// # Errors
    /// The OS error when the terminal cannot change mode.
    pub(super) fn start() -> io::Result<Self> {
        watch_signals();
        let raw = RawMode::enter()?;
        ON_SCREEN.store(true, Ordering::SeqCst);
        Ok(Self {
            keys: Keys::default(),
            raw: Some(raw),
        })
    }

    /// Asks for `what`, showing the answer as it is typed; the answer without surrounding
    /// spaces.
    ///
    /// # Errors
    /// [`Failure::Cancelled`] for Ctrl-C; [`Failure::Input`] for an answer over the limit, the
    /// end of input, or a terminal that fails.
    pub fn ask(&mut self, what: &str) -> Result<String, Failure> {
        let answer = self.line(&prompt_text(what), true)?;
        Ok(answer.trim().to_owned())
    }

    /// Asks for the secret `what`, showing nothing as it is typed.
    ///
    /// # Errors
    /// As [`Hidden::ask`].
    pub fn ask_secret(&mut self, what: &str) -> Result<Zeroizing<String>, Failure> {
        self.line(&prompt_text(what), false)
    }

    fn line(&mut self, prompt: &str, shown: bool) -> Result<Zeroizing<String>, Failure> {
        let mut screen = screen();
        let wrote = write!(screen, "{prompt}").and_then(|()| screen.flush());
        wrote.map_err(|error| terminal_failure(&error))?;
        let mut line = Line::new(MAX_ANSWER);
        let ending = self.read(&mut line, &mut screen, shown.then_some(prompt));
        let ended = write!(screen, "\r\n").and_then(|()| screen.flush());
        match ending.map_err(|error| terminal_failure(&error))? {
            Ending::Answered => ended
                .map(|()| line.into_text())
                .map_err(|error| terminal_failure(&error)),
            Ending::Interrupted => Err(Failure::Cancelled),
            Ending::TooLong => Err(Failure::Input {
                what: "the answer".to_owned(),
                reason: format!("it is longer than {MAX_ANSWER} bytes"),
            }),
            Ending::Ended => Err(Failure::Input {
                what: "the terminal".to_owned(),
                reason: "the input ended before an answer".to_owned(),
            }),
        }
    }

    /// Reads keys into `line` until it ends. With the `shown` prompt, keeps the answer on
    /// screen after it; without, shows nothing.
    fn read(
        &mut self,
        line: &mut Line,
        screen: &mut impl Write,
        shown: Option<&str>,
    ) -> io::Result<Ending> {
        let mut stdin = io::stdin().lock();
        let mut byte = [0_u8; 1];
        loop {
            match stdin.read(&mut byte) {
                Ok(0) => return Ok(Ending::Ended),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            let Some(key) = self.keys.push(byte[0]) else {
                continue;
            };
            let before = line.text().len();
            if let Some(ending) = line.apply(key) {
                return Ok(ending);
            }
            let Some(prompt) = shown else {
                continue;
            };
            let added = match key {
                Key::Char(character) if line.text().len() > before => Some(character),
                Key::Char(_) => continue,
                _ => None,
            };
            show(screen, prompt, line.text(), added)?;
        }
    }
}

impl Drop for Hidden {
    fn drop(&mut self) {
        drop(self.raw.take());
        ON_SCREEN.store(false, Ordering::SeqCst);
    }
}

/// Where questions are shown: the controlling terminal, so that neither a question nor a
/// shown answer lands in a redirected standard error; standard error when there is none.
fn screen() -> Box<dyn Write> {
    match File::options().write(true).open("/dev/tty") {
        Ok(terminal) => Box::new(terminal),
        Err(_) => Box::new(io::stderr()),
    }
}

/// Shows a change to a shown answer: the `added` character is printed; any other edit
/// redraws the line. The answer holds no control characters (the keys never give one).
fn show(screen: &mut impl Write, prompt: &str, text: &str, added: Option<char>) -> io::Result<()> {
    match added {
        Some(character) => write!(screen, "{character}")?,
        None => write!(screen, "\r\x1b[K{prompt}{text}")?,
    }
    screen.flush()
}

/// Makes `SIGINT`, `SIGTERM` and `SIGHUP` restore the terminal, then end the program with 128
/// plus the signal's number, as the default action would have. In raw mode the keys send no
/// signal, so these come from another program. They are watched on a thread of their own: a
/// question blocks the runtime's only thread while it waits for a key. Returns once the
/// signals are watched.
fn watch_signals() {
    SIGNALS.call_once(|| {
        let (ready, watched) = std::sync::mpsc::sync_channel::<()>(1);
        let spawned = std::thread::Builder::new()
            .name("anima-signals".to_owned())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                else {
                    return;
                };
                let number = runtime.block_on(first_signal(ready));
                RawMode::restore();
                if ON_SCREEN.load(Ordering::SeqCst) {
                    eprint!("\r\n");
                }
                std::process::exit(128 + number);
            });
        if spawned.is_ok() {
            // An error means the signals could not be watched; the question goes on without.
            let _watched = watched.recv();
        }
    });
}

/// The number of the first of `SIGINT`, `SIGTERM` and `SIGHUP`; says on `ready` once they
/// are watched. Never resolves when they cannot be.
async fn first_signal(ready: std::sync::mpsc::SyncSender<()>) -> i32 {
    let kinds = (
        signal(SignalKind::interrupt()),
        signal(SignalKind::terminate()),
        signal(SignalKind::hangup()),
    );
    let (Ok(mut interrupt), Ok(mut terminate), Ok(mut hangup)) = kinds else {
        drop(ready);
        return std::future::pending().await;
    };
    let _told = ready.send(());
    tokio::select! {
        _ = interrupt.recv() => 2,
        _ = terminate.recv() => 15,
        _ = hangup.recv() => 1,
    }
}
