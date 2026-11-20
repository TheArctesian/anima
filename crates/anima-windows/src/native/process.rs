//! Verify the owner and image of a running connection process without trusting a PID alone.
use super::identity::Identity;
use std::{io, path::Path};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    },
    core::PWSTR,
};

struct Process(HANDLE);
impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Whether PID still belongs to this user and runs exactly the expected trusted executable.
/// # Errors
/// Windows refuses inspection, the process exited, or the executable path cannot be read.
pub fn matches(pid: u32, expected: &Path) -> io::Result<bool> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(io::Error::other)?;
    let process = Process(handle);
    let current = Identity::current().map_err(io::Error::other)?;
    let owner = Identity::of_process(process.0).map_err(io::Error::other)?;
    if current.text().map_err(io::Error::other)? != owner.text().map_err(io::Error::other)? {
        return Ok(false);
    }
    let mut buffer = vec![0_u16; 32768];
    let mut size = u32::try_from(buffer.len()).map_err(io::Error::other)?;
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut size,
        )
    }
    .map_err(io::Error::other)?;
    let actual = String::from_utf16(&buffer[..size as usize]).map_err(io::Error::other)?;
    Ok(std::fs::canonicalize(actual)? == std::fs::canonicalize(expected)?)
}

/// The current account's SID, suitable for a per-user scheduled task principal.
/// # Errors
/// Windows cannot inspect the current process token or format its SID.
pub fn current_user() -> io::Result<String> {
    Identity::current()
        .and_then(|identity| identity.text())
        .map_err(io::Error::other)
}

/// Whether a process still runs. An absent PID is false; denied inspection is an error.
/// # Errors
/// Windows refuses to open or inspect the process.
pub fn running(pid: u32) -> io::Result<bool> {
    use windows::Win32::{
        Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    // SAFETY: the PID is only inspected; the returned handle is owned by Process.
    let handle = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
        Ok(handle) => handle,
        Err(error)
            if error.code() == windows::core::HRESULT::from_win32(ERROR_INVALID_PARAMETER.0) =>
        {
            return Ok(false);
        }
        Err(error) => return Err(io::Error::other(error)),
    };
    let process = Process(handle);
    // SAFETY: process keeps the synchronization handle open; the zero timeout never waits.
    match unsafe { WaitForSingleObject(process.0, 0) } {
        WAIT_TIMEOUT => Ok(true),
        WAIT_OBJECT_0 => Ok(false),
        _ => Err(io::Error::last_os_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_expected_process_image_is_accepted() {
        assert!(running(std::process::id()).unwrap());
        assert!(!running(0).unwrap());
        let executable = std::env::current_exe().unwrap();
        assert!(matches(std::process::id(), &executable).unwrap());
        assert!(!matches(std::process::id(), &std::env::temp_dir()).unwrap());
        assert!(matches(0, &executable).is_err());
    }
}
