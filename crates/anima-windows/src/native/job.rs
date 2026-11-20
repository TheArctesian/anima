//! A child starts suspended, joins a kill-on-close job, then resumes. No descendant can
//! escape between spawn and assignment. The caller retains its child handle until attached.
use std::{
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            Threading::{
                OpenProcess, OpenThread, PROCESS_SET_QUOTA, PROCESS_TERMINATE, ResumeThread,
                THREAD_SUSPEND_RESUME,
            },
        },
    },
    core::PCWSTR,
};

#[derive(Debug)]
struct Handle(OwnedHandle);
impl Handle {
    fn new(handle: HANDLE) -> Self {
        // SAFETY: callers pass a newly returned owned handle, never borrowed or duplicated.
        Self(unsafe { OwnedHandle::from_raw_handle(handle.0) })
    }
    fn raw(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }
}

/// Creation flags a caller must apply before passing its suspended child to `Job::start`.
pub const SUSPENDED: u32 = 0x0000_0004 | 0x0800_0000;

/// Owns a native job. Dropping it terminates the child and every descendant still running.
#[derive(Debug)]
pub struct Job(Handle);

mod child;
pub use child::Child;

impl Job {
    /// Open a process only if it is still a member of this owned job.
    /// # Errors
    /// The process is absent, cannot be inspected, or belongs to another job.
    pub fn child(&self, pid: u32) -> io::Result<Child> {
        child::open(self.0.raw(), pid)
    }
    /// Assign a newly created, suspended child and resume its initial thread.
    /// The caller must keep ownership of the child and kill it if this returns an error.
    /// # Errors
    /// Windows cannot create/configure the job, assign the child, or resume its thread.
    pub fn start(pid: u32) -> io::Result<Self> {
        // SAFETY: no borrowed security attributes or name; the returned handle is owned.
        let job = Handle::new(
            unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(io::Error::other)?,
        );
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: limits lives through the call, with its exact structure size.
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                    .map_err(io::Error::other)?,
            )
        }
        .map_err(io::Error::other)?;
        // SAFETY: PID names the caller's still-owned suspended child; rights are limited.
        let process = Handle::new(
            unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) }
                .map_err(io::Error::other)?,
        );
        // SAFETY: both handles remain owned throughout the call.
        unsafe { AssignProcessToJobObject(job.raw(), process.raw()) }.map_err(io::Error::other)?;
        resume(pid)?;
        Ok(Self(job))
    }

    /// End the entire job now. Dropping the returned empty owner does nothing further.
    pub fn kill(self) {
        drop(self.0);
    }
}

fn resume(pid: u32) -> io::Result<()> {
    // SAFETY: this snapshots thread identifiers and returns an owned snapshot handle.
    let snapshot = Handle::new(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.map_err(io::Error::other)?,
    );
    let mut entry = THREADENTRY32 {
        dwSize: u32::try_from(size_of::<THREADENTRY32>()).map_err(io::Error::other)?,
        ..Default::default()
    };
    // SAFETY: entry is initialized with its size and remains writable through each call.
    let mut next = unsafe { Thread32First(snapshot.raw(), &raw mut entry) };
    while next.is_ok() {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: the child has not run; its initial thread cannot exit/reuse this ID.
            let thread = Handle::new(
                unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }
                    .map_err(io::Error::other)?,
            );
            // SAFETY: thread is an owned, open handle to the suspended child's thread.
            if unsafe { ResumeThread(thread.raw()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        // SAFETY: same owned snapshot and initialized entry as Thread32First.
        next = unsafe { Thread32Next(snapshot.raw(), &raw mut entry) };
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "suspended child thread not found",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::windows::process::CommandExt,
        process::Command,
        time::{Duration, Instant},
    };
    use windows::Win32::{
        Foundation::WAIT_OBJECT_0,
        System::Threading::{PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };

    #[test]
    fn job_fixture() {
        let Ok(mode) = std::env::var("ANIMA_JOB_FIXTURE") else {
            return;
        };
        if mode == "root" {
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "native::job::tests::job_fixture", "--nocapture"])
                .env("ANIMA_JOB_FIXTURE", "leaf")
                .spawn()
                .unwrap();
            std::fs::write(
                std::env::var_os("ANIMA_JOB_MARKER").unwrap(),
                child.id().to_string(),
            )
            .unwrap();
            let _ = child.wait();
        } else {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn closing_a_job_terminates_its_child_and_descendant() {
        let folder = tempfile::tempdir().unwrap();
        let marker = folder.path().join("descendant");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "native::job::tests::job_fixture", "--nocapture"])
            .env("ANIMA_JOB_FIXTURE", "root")
            .env("ANIMA_JOB_MARKER", &marker)
            .creation_flags(SUSPENDED)
            .spawn()
            .unwrap();
        let job = Job::start(child.id()).unwrap();
        assert!(job.child(std::process::id()).is_err());
        let mut held = job.child(child.id()).unwrap();
        assert_eq!(held.id(), child.id());
        assert!(held.try_wait().unwrap().is_none());
        let end = Instant::now() + Duration::from_secs(5);
        let pid = loop {
            if let Ok(text) = std::fs::read_to_string(&marker)
                && let Ok(pid) = text.parse::<u32>()
            {
                break pid;
            }
            assert!(Instant::now() < end, "child did not start its descendant");
            std::thread::sleep(Duration::from_millis(20));
        };
        // SAFETY: opens only a synchronization handle to the still-running fixture child.
        let descendant =
            Handle::new(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.unwrap());
        job.kill();
        // SAFETY: owned synchronization handle remains valid through the bounded wait.
        assert_eq!(
            unsafe { WaitForSingleObject(descendant.raw(), 5000) },
            WAIT_OBJECT_0
        );
        let end = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < end, "job leader survived closing the job");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(held.wait().is_ok());
    }
}
