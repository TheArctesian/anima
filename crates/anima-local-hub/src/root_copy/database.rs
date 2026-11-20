//! The dump of the database of a stopped hub: `anima-database serve` runs alone for the time of
//! one `pg_dump`, then stops, so the files of the cluster are at rest when they are copied.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anima_database::{Tree, backup, startup};
#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;

use super::{RootCopyError, io_error};
use crate::{Carried, Folders, members};

/// A start after a kill replays the write-ahead log first.
const READY_LIMIT: Duration = Duration::from_secs(300);
/// `anima-database serve` gives its server 30 seconds to stop.
const STOP_LIMIT: Duration = Duration::from_secs(45);
const PAUSE: Duration = Duration::from_millis(100);

/// The database program while it serves for the dump. A drop that comes before `stop` kills it.
struct Serving(Child);

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts the database of `folders` alone, writes its dump to `file` and stops the database.
pub(super) fn dump(carried: &Carried, folders: &Folders, file: &Path) -> Result<(), RootCopyError> {
    let tree = Tree::at(carried.tree())?;
    let spec = members::database(carried, folders);
    let mut command = Command::new(&spec.program);
    command.env_clear();
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let child = command
        .args(&spec.args)
        .stdin(if cfg!(windows) {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(io_error("start the database for the dump"))?;
    let mut serving = Serving(child);
    wait_until_ready(&mut serving, &tree, folders)?;
    let dumped = backup::write(&tree, folders.database(), file);
    stop(serving)?;
    Ok(dumped?)
}

fn wait_until_ready(
    serving: &mut Serving,
    tree: &Tree,
    folders: &Folders,
) -> Result<(), RootCopyError> {
    let started = Instant::now();
    loop {
        let ended = serving
            .0
            .try_wait()
            .map_err(io_error("wait for the database"))?;
        if let Some(status) = ended {
            return Err(RootCopyError::Database(format!("it ended with {status}")));
        }
        let ready = startup::status(tree, folders.database())
            .is_ok_and(|status| status.server_accepts_connections)
            && folders
                .database()
                .url_file(anima_database::Role::Owner)
                .is_file();
        if ready {
            return Ok(());
        }
        if started.elapsed() > READY_LIMIT {
            return Err(RootCopyError::Database(
                "it was not ready in 300 seconds".to_owned(),
            ));
        }
        thread::sleep(PAUSE);
    }
}

/// Asks the database to stop and waits for it: the files must be at rest before the copy.
fn stop(mut serving: Serving) -> Result<(), RootCopyError> {
    #[cfg(unix)]
    {
        let process = i32::try_from(serving.0.id())
            .map_err(|_| RootCopyError::Database("its process number does not fit".to_owned()))?;
        kill(Pid::from_raw(process), Signal::SIGTERM)
            .map_err(|_| RootCopyError::Database("it took no stop signal".to_owned()))?;
    }
    #[cfg(windows)]
    {
        use std::io::Write;
        serving
            .0
            .stdin
            .as_mut()
            .ok_or_else(|| RootCopyError::Database("no stop pipe".into()))?
            .write_all(b"stop\n")
            .map_err(io_error("ask the database to stop"))?;
    }
    let asked = Instant::now();
    loop {
        let ended = serving
            .0
            .try_wait()
            .map_err(io_error("wait for the database to stop"))?;
        if ended.is_some() {
            return Ok(());
        }
        if asked.elapsed() > STOP_LIMIT {
            return Err(RootCopyError::Database(
                "it did not stop in 45 seconds".to_owned(),
            ));
        }
        thread::sleep(PAUSE);
    }
}
