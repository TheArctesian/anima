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

/// Attach a GUI-subsystem command to its caller's console, preserving redirected pipes.
/// Call before any Rust standard I/O is used. A scheduled task normally has no parent console.
/// # Errors
/// There is no parent console, the process is already attached, or restoration fails.
pub fn attach_parent() -> io::Result<()> {
    use windows::Win32::{
        Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType},
        System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
        },
    };
    let redirected: Vec<_> = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE]
        .into_iter()
        .filter_map(|name| {
            // SAFETY: borrowed standard handles are inspected, never closed or dereferenced.
            let handle = unsafe { GetStdHandle(name) }.ok()?;
            // SAFETY: GetFileType accepts the borrowed handle and reports invalid handles as unknown.
            matches!(
                unsafe { GetFileType(handle) },
                FILE_TYPE_DISK | FILE_TYPE_PIPE
            )
            .then_some((name, handle))
        })
        .collect();
    // SAFETY: attaches this process to its parent without allocating a console window.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.map_err(io::Error::other)?;
    for (name, handle) in redirected {
        // SAFETY: redirected handles belong to this process and remain open across attachment.
        unsafe { SetStdHandle(name, handle) }.map_err(io::Error::other)?;
    }
    Ok(())
}
