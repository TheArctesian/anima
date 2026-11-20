//! Byte-oriented terminal input and exact restoration for prompts and SSH sessions.
use std::{io, sync::Mutex};
use windows::Win32::{
    Foundation::HANDLE,
    System::Console::{
        CONSOLE_MODE, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
        ENABLE_VIRTUAL_TERMINAL_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE,
        SetConsoleMode,
    },
};

// Store the handle value, not a borrowed pointer; the standard input handle belongs to
// the process and is never closed here. A mutex serializes signal/panic restoration.
static SAVED: Mutex<Option<(isize, CONSOLE_MODE)>> = Mutex::new(None);

/// Disable echo and line processing, and deliver arrow/function keys as VT bytes.
/// # Errors
/// Standard input is not a console or the mode cannot be changed.
pub fn enter() -> io::Result<()> {
    let mut saved = SAVED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if saved.is_some() {
        return Err(io::Error::other("terminal raw mode is already active"));
    }
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.map_err(io::Error::other)?;
    let mut original = CONSOLE_MODE::default();
    unsafe { GetConsoleMode(handle, &raw mut original) }.map_err(io::Error::other)?;
    let mode = (original & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT))
        | ENABLE_VIRTUAL_TERMINAL_INPUT;
    unsafe { SetConsoleMode(handle, mode) }.map_err(io::Error::other)?;
    *saved = Some((handle.0 as isize, original));
    Ok(())
}

/// Restore the exact input mode from before `enter`; safe to call more than once.
/// # Errors
/// Windows refuses restoration. The saved mode is retained so a later call can retry.
pub fn restore() -> io::Result<()> {
    let mut saved = SAVED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((handle, mode)) = *saved {
        unsafe { SetConsoleMode(HANDLE(handle as *mut _), mode) }.map_err(io::Error::other)?;
        *saved = None;
    }
    Ok(())
}
