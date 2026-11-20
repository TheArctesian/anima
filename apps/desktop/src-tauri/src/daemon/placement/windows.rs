//! Protected Windows executable placement; stop the previous task before replacing its image.
use super::run::{Run, daemon_env, run};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub const DAEMON_FILE: &str = "anima-daemon.exe";
const SIZE_LIMIT: u64 = 256 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlacementError {
    #[error("a daemon installation folder can be changed by another user")]
    Untrusted,
    #[error("the bundled daemon is not a trusted regular file of a sane size")]
    Source,
    #[error("the daemon could not be copied or replaced")]
    Write,
}

pub fn destination(home: &Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join("AppData/Local"))
        .join("Anima/bin")
        .join(DAEMON_FILE)
}

pub fn bundled_beside(executable: &Path) -> Option<PathBuf> {
    Some(executable.parent()?.join(DAEMON_FILE))
}

pub fn place(source: &Path, destination: &Path) -> Result<PathBuf, PlacementError> {
    let mut input =
        anima_windows::files::read(source, false).map_err(|_| PlacementError::Source)?;
    if input.metadata().map_err(|_| PlacementError::Source)?.len() > SIZE_LIMIT {
        return Err(PlacementError::Source);
    }
    let folder = destination.parent().ok_or(PlacementError::Untrusted)?;
    anima_windows::files::directory(folder).map_err(|_| PlacementError::Untrusted)?;
    anima_windows::files::protect(folder).map_err(|_| PlacementError::Untrusted)?;
    let mut salt = [0_u8; 8];
    getrandom::fill(&mut salt).map_err(|_| PlacementError::Write)?;
    let part = folder.join(format!(".daemon-{:016x}.part", u64::from_le_bytes(salt)));
    let mut stopped = false;
    let result = copy(&mut input, &part).and_then(|()| {
        if destination.exists() {
            task("/End")?;
            stopped = true;
        }
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            match fs::rename(&part, destination) {
                Err(error)
                    if error.kind() == io::ErrorKind::PermissionDenied && Instant::now() < end =>
                {
                    std::thread::sleep(Duration::from_millis(100));
                }
                result => return result,
            }
        }
    });
    if result.is_err() {
        let _ = fs::remove_file(&part);
        if stopped {
            let _ = task("/Run");
        }
    }
    result.map_err(|_| PlacementError::Write)?;
    Ok(destination.to_owned())
}

fn copy(input: &mut fs::File, part: &Path) -> io::Result<()> {
    let mut output = OpenOptions::new().write(true).create_new(true).open(part)?;
    anima_windows::files::protect(part)?;
    if io::copy(&mut input.take(SIZE_LIMIT + 1), &mut output)? > SIZE_LIMIT {
        return Err(io::ErrorKind::FileTooLarge.into());
    }
    output.flush()?;
    output.sync_all()
}

fn task(action: &str) -> io::Result<()> {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(io::ErrorKind::NotFound)?;
    let program = root.join("System32/schtasks.exe");
    let name = format!(
        "Anima.ing-Daemon-{}",
        anima_windows::process::current_user()?
    );
    let env = daemon_env(&root, |key| std::env::var_os(key));
    let stop = AtomicBool::new(false);
    let request = Run {
        program: &program,
        args: &[action, "/TN", &name],
        input: None,
        env: &env,
        deadline: Duration::from_secs(30),
        stop: &stop,
    };
    // An installation may exist with no registered task. A locked image still refuses replacement.
    match run(&request) {
        Ok(()) => Ok(()),
        Err(super::run::RunError::Failed) if action == "/End" => Ok(()),
        Err(_) => Err(io::Error::other(
            "the previous daemon task could not be controlled",
        )),
    }
}
