//! Identify and stop the former Unix server before admitting a new owner.
use super::{Limits, Lock, ServerError, io_error};
use crate::{Layout, Program, Tree};
use std::{ffi::OsStr, fs, io, path::Path, process::Command};

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
pub(super) fn is_postgres_on(command_line: &str, cluster: &Path) -> bool {
    let flag = format!(" -D {}", cluster.display());
    let Some((program, after)) = command_line.split_once(&flag) else {
        return false;
    };
    (after.is_empty() || after.starts_with(' '))
        && Path::new(program.trim()).file_name() == Some(OsStr::new("postgres"))
}
