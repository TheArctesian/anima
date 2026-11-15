//! The `postgres` process of one bundled database: one copy at a time, started as a child,
//! proved ready, watched, and stopped with a fast shutdown.

use std::ffi::OsStr;
use std::fs::{self, File, TryLockError};
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use crate::{Layout, Program, Tree, owner_only};

mod pid_file;
#[cfg(test)]
mod tests;

use pid_file::PidFile;

/// How often a wait looks again.
const POLL: Duration = Duration::from_millis(10);

/// The least time an immediate shutdown gets before the process is killed.
const QUIT_LIMIT: Duration = Duration::from_secs(5);

/// How long a dropped server gets for its fast shutdown.
const DROP_LIMIT: Duration = Duration::from_secs(30);

/// The server log moves aside at a start when it is larger than this.
const LOG_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("another copy already runs this database")]
    AnotherCopy,
    #[error("could not {step}")]
    Io {
        step: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("the server ended while it started ({status}); see the server log")]
    EndedDuringStart { status: ExitStatus },
    #[error("the server was not ready after {} s", limit.as_secs())]
    NotReady { limit: Duration },
    #[error("a server that an earlier copy left did not stop")]
    LeftOverStays,
}

fn io_error(step: &'static str) -> impl FnOnce(io::Error) -> ServerError {
    move |source| ServerError::Io { step, source }
}

/// The server's settings for one computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub max_connections: u32,
    pub shared_buffers_mb: u32,
    /// Ask the drive to empty its cache at each commit (`fsync_writethrough`). macOS needs it
    /// to keep the cluster whole through a power cut; it makes a commit slower.
    pub flush_through_drive_cache: bool,
}

/// How long the waits may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub ready: Duration,
    pub stop: Duration,
}

/// Held by the one copy that runs this database. The operating system ends the lock with the
/// process, so a killed copy never leaves it held.
#[derive(Debug)]
pub struct Lock(#[expect(dead_code, reason = "the open file is the lock")] File);

/// Takes the lock of `layout`'s database, and makes the root folder first when it is missing.
///
/// Without it two copies that start together both make the cluster, both fail, and each
/// removes the other's half-made folder.
///
/// # Errors
/// [`ServerError::AnotherCopy`] when the lock is held.
pub fn lock(layout: &Layout) -> Result<Lock, ServerError> {
    owner_only::folder(layout.root()).map_err(io_error("make the database root"))?;
    let file = owner_only::appended_file(&layout.lock_file()).map_err(io_error("open the lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Lock(file)),
        Err(TryLockError::WouldBlock) => Err(ServerError::AnotherCopy),
        Err(TryLockError::Error(source)) => Err(io_error("take the lock")(source)),
    }
}

/// Stops a server that a killed copy left running on this cluster; `true` when one was there.
/// The caller holds the [`Lock`], so no other copy starts a server meanwhile.
///
/// A signal goes only to this cluster's PostgreSQL server: one that answers on this
/// database's own socket, or a `postgres` process that was started on this cluster. The
/// process ID in a stale `postmaster.pid` can belong to any program by now; then the file is
/// removed, because `postgres` refuses to start while the number in it is a live process of
/// the same user.
///
/// # Errors
/// When `postmaster.pid` cannot be read, a program cannot run, or a server is there and does
/// not stop in `limits.stop`.
pub fn stop_left_over(
    _lock: &Lock,
    tree: &Tree,
    layout: &Layout,
    limits: Limits,
) -> Result<bool, ServerError> {
    let pid_file = layout.cluster().join("postmaster.pid");
    let text = match fs::read_to_string(&pid_file) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(io_error("read postmaster.pid")(source)),
    };
    if !answers_on_socket(tree, layout)? && !names_this_clusters_server(&text, layout)? {
        fs::remove_file(&pid_file).map_err(io_error("remove a stale postmaster.pid"))?;
        return Ok(false);
    }
    let stopped = tree
        .command(Program::PgCtl)
        .arg("stop")
        .arg("--pgdata")
        .arg(layout.cluster())
        .args(["--mode", "fast", "--wait", "--silent"])
        .arg("--timeout")
        .arg(limits.stop.as_secs().max(1).to_string())
        .status()
        .map_err(io_error("run pg_ctl"))?;
    if stopped.success() {
        Ok(true)
    } else {
        Err(ServerError::LeftOverStays)
    }
}

fn answers_on_socket(tree: &Tree, layout: &Layout) -> Result<bool, ServerError> {
    let answer = tree
        .command(Program::PgIsready)
        .arg("--host")
        .arg(layout.socket_folder())
        .args(["--timeout", "3", "--quiet"])
        .status()
        .map_err(io_error("run pg_isready"))?;
    // 0: accepts connections. 1: answers, and refuses for now (it starts or stops).
    Ok(matches!(answer.code(), Some(0 | 1)))
}

/// Whether the process that `postmaster.pid` names is now a `postgres` that was started on
/// this cluster. Another cluster's server can have the number a dead server left.
fn names_this_clusters_server(pid_file_text: &str, layout: &Layout) -> Result<bool, ServerError> {
    let Some(process_id) = pid_file_text
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
    else {
        return Ok(false);
    };
    let output = Command::new("ps")
        .env_clear()
        .env("PATH", "/bin:/usr/bin")
        .args(["-o", "args=", "-p"])
        .arg(process_id.to_string())
        .output()
        .map_err(io_error("run ps"))?;
    let command_line = String::from_utf8_lossy(&output.stdout);
    let command_line = command_line.trim();
    match output.status.code() {
        // `ps` ends with 1 and prints nothing when no such process is there.
        Some(1) if command_line.is_empty() => Ok(false),
        Some(0) => Ok(is_postgres_on(command_line, &layout.cluster())),
        _ => Err(io_error("run ps")(io::Error::other(format!(
            "ps ended with {}",
            output.status
        )))),
    }
}

/// Whether `command_line` is a `postgres` program with `-D <cluster>`, as [`Server::start`]
/// and `pg_ctl` start it. The cluster's path must end there: `cluster-copy` is another one.
fn is_postgres_on(command_line: &str, cluster: &Path) -> bool {
    let flag = format!(" -D {}", cluster.display());
    let Some((program, after)) = command_line.split_once(&flag) else {
        return false;
    };
    (after.is_empty() || after.starts_with(' '))
        && Path::new(program.trim()).file_name() == Some(OsStr::new("postgres"))
}

/// Whether a server of this cluster accepts connections now, read from `postmaster.pid`
/// with no lock taken, so a look never stands in the way of a copy that starts.
///
/// # Errors
/// When `postmaster.pid` is there and cannot be read.
pub fn accepts_connections(layout: &Layout) -> Result<bool, ServerError> {
    let text = match fs::read_to_string(layout.cluster().join("postmaster.pid")) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(io_error("read postmaster.pid")(source)),
    };
    let process = PidFile::parse(&text)
        .and_then(PidFile::ready_process)
        .and_then(|id| i32::try_from(id).ok());
    // Signal 0 sends nothing and tells if the process is there.
    Ok(process.is_some_and(|id| kill(Pid::from_raw(id), None).is_ok()))
}

/// How a stop went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// A fast shutdown: sessions ended, a checkpoint written, no recovery at the next start.
    Clean,
    /// The fast shutdown took longer than the limit, so the server was told to quit at once.
    /// The next start recovers from the write-ahead log.
    Immediate,
}

/// A running `postgres` that this process started and owns.
#[derive(Debug)]
pub struct Server {
    child: Child,
    spawned_at: SystemTime,
}

impl Server {
    /// Starts `postgres` on `layout`'s cluster as a child of this process, for the copy that
    /// holds the [`Lock`]. The server listens only on the Unix socket (DB-001) and its log
    /// holds no statement and no error message of a session (LOG-007).
    ///
    /// # Errors
    /// When the socket folder or the log cannot be prepared, or the program cannot start.
    pub fn start(
        _lock: &Lock,
        tree: &Tree,
        layout: &Layout,
        settings: Settings,
    ) -> Result<Self, ServerError> {
        owner_only::folder(layout.socket_folder()).map_err(io_error("make the socket folder"))?;
        let log = open_log(layout).map_err(io_error("open the server log"))?;
        let errors = log.try_clone().map_err(io_error("open the server log"))?;
        let mut command = tree.command(Program::Postgres);
        command.arg("-D").arg(layout.cluster());
        for setting in arguments(layout, settings) {
            command.arg("-c").arg(setting);
        }
        let spawned_at = SystemTime::now();
        let child = command
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(errors)
            .spawn()
            .map_err(io_error("start postgres"))?;
        Ok(Self { child, spawned_at })
    }

    #[must_use]
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    /// Waits until this server accepts connections.
    ///
    /// Ready means that `postmaster.pid` names this child, was written after the child started
    /// and says `ready`. An answer on the socket is not enough: a server that an earlier copy
    /// left can answer there while this child fails to start.
    ///
    /// # Errors
    /// When the child ends first, or is not ready in `limits.ready`.
    pub fn wait_until_ready(&mut self, layout: &Layout, limits: Limits) -> Result<(), ServerError> {
        let started = Instant::now();
        let pid_file = layout.cluster().join("postmaster.pid");
        loop {
            if let Some(status) = self.ended()? {
                return Err(ServerError::EndedDuringStart { status });
            }
            let ready = fs::read_to_string(&pid_file)
                .ok()
                .and_then(|text| PidFile::parse(&text))
                .is_some_and(|file| file.is_ready_for(self.process_id(), self.spawned_at));
            if ready {
                return Ok(());
            }
            if started.elapsed() > limits.ready {
                return Err(ServerError::NotReady {
                    limit: limits.ready,
                });
            }
            std::thread::sleep(POLL);
        }
    }

    /// The exit status when the server has ended by itself, `None` while it runs.
    ///
    /// # Errors
    /// When the operating system cannot say.
    pub fn ended(&mut self) -> Result<Option<ExitStatus>, ServerError> {
        self.child
            .try_wait()
            .map_err(io_error("look at the server process"))
    }

    /// A fast shutdown, and after `limits.stop` an immediate one. A server that had ended
    /// by itself before the call counts as [`Stopped::Clean`]: nothing was cut short here,
    /// and [`Server::ended`] is the way to learn of a server that died.
    ///
    /// # Errors
    /// When the server cannot be signalled or waited for.
    pub fn stop(mut self, limits: Limits) -> Result<Stopped, ServerError> {
        self.end(limits.stop)
    }

    fn end(&mut self, limit: Duration) -> Result<Stopped, ServerError> {
        if self.ended()?.is_some() {
            return Ok(Stopped::Clean);
        }
        self.signal(Signal::SIGINT)?;
        if self.ends_within(limit)? {
            return Ok(Stopped::Clean);
        }
        self.signal(Signal::SIGQUIT)?;
        if !self.ends_within(limit.max(QUIT_LIMIT))? {
            // It ignores both requests. The operating system ends it; the cluster recovers.
            self.child.kill().map_err(io_error("kill the server"))?;
            self.child
                .wait()
                .map_err(io_error("wait for the server to end"))?;
        }
        Ok(Stopped::Immediate)
    }

    fn ends_within(&mut self, limit: Duration) -> Result<bool, ServerError> {
        let started = Instant::now();
        loop {
            if self.ended()?.is_some() {
                return Ok(true);
            }
            if started.elapsed() >= limit {
                return Ok(false);
            }
            std::thread::sleep(POLL);
        }
    }

    fn signal(&self, signal: Signal) -> Result<(), ServerError> {
        let pid = i32::try_from(self.process_id()).map_err(|_| ServerError::Io {
            step: "signal the server",
            source: io::Error::other("the process ID does not fit"),
        })?;
        kill(Pid::from_raw(pid), signal).map_err(|errno| ServerError::Io {
            step: "signal the server",
            source: io::Error::from(errno),
        })
    }
}

/// A server never outlives the value that owns it: a panic or an early return in the caller
/// must not leave a `postgres` running. The wait has a limit, as every stop has.
impl Drop for Server {
    fn drop(&mut self) {
        // Nothing can be reported from a drop; `stop` is the call that reports.
        self.end(DROP_LIMIT).ok();
    }
}

/// The settings on the command line, so nothing depends on an edited `postgresql.conf`.
fn arguments(layout: &Layout, settings: Settings) -> Vec<String> {
    let mut arguments = vec![
        "listen_addresses=".to_owned(),
        format!(
            "unix_socket_directories={}",
            layout.socket_folder().display()
        ),
        "unix_socket_permissions=0700".to_owned(),
        "port=5432".to_owned(),
        format!("max_connections={}", settings.max_connections),
        format!("shared_buffers={}MB", settings.shared_buffers_mb),
        "dynamic_shared_memory_type=posix".to_owned(),
        // LOG-007. A statement can hold a password, and so can the message of an error:
        // a syntax error quotes the text near it, a failed cast quotes the value, a
        // constraint quotes the row. So no statement and no ERROR of a session is logged;
        // the session itself still gets its error. LOG, FATAL and PANIC stay: starts, stops,
        // recovery, refused logins. None of these can be changed by the hub's login.
        "log_statement=none".to_owned(),
        "log_min_duration_statement=-1".to_owned(),
        "log_min_duration_sample=-1".to_owned(),
        "log_transaction_sample_rate=0".to_owned(),
        "log_min_error_statement=panic".to_owned(),
        "log_min_messages=log".to_owned(),
        "log_error_verbosity=terse".to_owned(),
    ];
    if settings.flush_through_drive_cache {
        arguments.push("wal_sync_method=fsync_writethrough".to_owned());
    }
    arguments
}

/// The server log, open for appending. A log above [`LOG_LIMIT`] becomes `server.log.1`
/// first, in place of the one before it.
fn open_log(layout: &Layout) -> io::Result<File> {
    let path = layout.server_log();
    if fs::metadata(&path).is_ok_and(|log| log.len() > LOG_LIMIT) {
        let mut older = path.clone().into_os_string();
        older.push(".1");
        fs::rename(&path, older)?;
    }
    owner_only::appended_file(&path)
}
