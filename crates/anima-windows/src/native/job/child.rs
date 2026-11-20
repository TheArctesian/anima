//! A held process handle, admitted only through membership of our private job.
use super::Handle;
use std::{io, os::windows::process::ExitStatusExt, process::ExitStatus};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        JobObjects::IsProcessInJob,
        Threading::{
            GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            PROCESS_SYNCHRONIZE, WaitForSingleObject,
        },
    },
};

#[derive(Debug)]
pub struct Child {
    handle: Handle,
    pid: u32,
}

pub(super) fn open(job: HANDLE, pid: u32) -> io::Result<Child> {
    // SAFETY: inspection only; the returned handle is owned for its whole use.
    let handle = Handle::new(
        unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }
        .map_err(io::Error::other)?,
    );
    let mut member = windows::core::BOOL::default();
    // SAFETY: both handles and the writable result outlive the call.
    unsafe { IsProcessInJob(handle.raw(), Some(job), &raw mut member) }
        .map_err(io::Error::other)?;
    if !member.as_bool() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(Child { handle, pid })
}

impl Child {
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.pid
    }

    /// Inspect the held process, without trusting a PID that could have been reused.
    /// # Errors
    /// Windows cannot wait on or read the owned process.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.wait_for(0)
    }

    /// Wait until the held process ends. Kill its owning job first for bounded shutdown.
    /// # Errors
    /// Windows cannot wait on or read the owned process.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.wait_for(INFINITE)?
            .ok_or_else(|| io::ErrorKind::TimedOut.into())
    }

    fn wait_for(&self, timeout: u32) -> io::Result<Option<ExitStatus>> {
        // SAFETY: the owned process handle remains valid through both calls.
        match unsafe { WaitForSingleObject(self.handle.raw(), timeout) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut status = 0;
                unsafe { GetExitCodeProcess(self.handle.raw(), &raw mut status) }
                    .map_err(io::Error::other)?;
                Ok(Some(ExitStatus::from_raw(status)))
            }
            _ => Err(io::Error::last_os_error()),
        }
    }
}
