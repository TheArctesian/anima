//! The terminal's raw mode, always undone: `anima ssh` (K4 Section 8) and the hidden prompts
//! (CMD-010) enter it. `crossterm` does the `unsafe` work, so this crate keeps `unsafe_code`
//! forbidden.

use std::io;
use std::sync::Once;

use crossterm::terminal;

/// Raw mode while it lives: no line editing, no echo, no signals from keys; keys arrive as
/// they are. Dropping it restores the terminal's previous mode, on every return, error and
/// unwinding panic.
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

    /// Restores the previous mode when raw mode is on; does nothing otherwise. For an exit
    /// that skips destructors.
    pub fn restore() {
        let _restored = terminal::disable_raw_mode();
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        Self::restore();
    }
}
