//! Windows PostgreSQL lifetime: one owned job for the server tree, and `pg_ctl` for a
//! normal fast shutdown. Never signal a recycled PID from an unproved stale file.
use super::{Limits, Lock, Server, ServerError, Stopped, io_error};
use crate::{Layout, Program, Tree};
use std::{
    fs, io,
    process::Stdio,
    time::{Duration, Instant},
};

mod start;

pub fn stop_left_over(
    _: &Lock,
    tree: &Tree,
    layout: &Layout,
    _: Limits,
) -> Result<bool, ServerError> {
    let file = layout.cluster().join("postmaster.pid");
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_error("read postmaster.pid")(error)),
    };
    let pid = text
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok());
    let live = pid
        .map(anima_windows::process::running)
        .transpose()
        .map_err(io_error("inspect the former server"))?
        .unwrap_or(false);
    let same = if let Some(pid) = pid.filter(|_| live) {
        anima_windows::process::matches(pid, &tree.path_of(Program::Postgres))
            .map_err(io_error("inspect the former server image"))?
    } else {
        false
    };
    if same {
        // Our job cannot leave a live server behind. Without an owned process handle we
        // cannot prove that this instance is this cluster, so do not stop another one.
        return Err(ServerError::LeftOverStays);
    }
    fs::remove_file(file).map_err(io_error("remove a stale postmaster.pid"))?;
    Ok(false)
}

impl Server {
    pub(super) fn end(&mut self, limit: Duration) -> Result<Stopped, ServerError> {
        if self.ended()?.is_some() {
            self.job.take();
            return Ok(Stopped::Clean);
        }
        let mut stop = self
            .tree
            .command(Program::PgCtl)
            .args(["stop", "--mode", "fast", "--wait", "--silent", "--timeout"])
            .arg(limit.as_secs().max(1).to_string())
            .arg("--pgdata")
            .arg(self.layout.cluster())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(io_error("start pg_ctl"))?;
        let end = Instant::now() + limit + Duration::from_secs(1);
        loop {
            if let Some(status) = stop.try_wait().map_err(io_error("wait for pg_ctl"))? {
                if status.success() && self.ended()?.is_some() {
                    self.job.take();
                    return Ok(Stopped::Clean);
                }
                break;
            }
            if Instant::now() >= end {
                let _ = stop.kill();
                let _ = stop.wait();
                break;
            }
            std::thread::sleep(super::POLL);
        }
        self.job.take();
        self.child
            .wait()
            .map_err(io_error("wait for the server to end"))?;
        Ok(Stopped::Immediate)
    }
}
