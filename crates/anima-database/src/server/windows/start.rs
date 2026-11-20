//! `pg_ctl` drops administrative groups before starting PostgreSQL. Its whole descendant
//! tree joins our job before any code runs; the server handle is admitted by job membership.
use super::super::{Lock, Server, ServerError, Settings, arguments, io_error, open_log};
use crate::{Layout, Program, Tree, windows::Endpoint};
use anima_windows::job::Job;
use std::{
    fs,
    os::windows::process::CommandExt,
    process::Stdio,
    time::{Duration, Instant, SystemTime},
};

const START_LIMIT: Duration = Duration::from_secs(30);

impl Server {
    /// Start the native server with PostgreSQL's restricted token and an owned job.
    /// # Errors
    /// Startup, privilege reduction or proving process ownership fails.
    pub fn start(
        _: &Lock,
        tree: &Tree,
        layout: &Layout,
        settings: Settings,
    ) -> Result<Self, ServerError> {
        let log = open_log(layout).map_err(io_error("open the server log"))?;
        let errors = log.try_clone().map_err(io_error("open the server log"))?;
        let mut settings = arguments(layout, settings);
        settings.extend(
            Endpoint::load(layout)
                .and_then(|endpoint| endpoint.settings())
                .map_err(io_error("read the database endpoint"))?,
        );
        let options = settings
            .iter()
            .map(|value| format!("-c {}", quote(value)))
            .collect::<Vec<_>>()
            .join(" ");
        let spawned_at = SystemTime::now();
        let mut launcher = tree
            .command(Program::PgCtl)
            .args(["start", "--no-wait", "--silent", "--pgdata"])
            .arg(layout.cluster())
            .arg("--options")
            .arg(options)
            .creation_flags(anima_windows::job::SUSPENDED)
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(errors)
            .spawn()
            .map_err(io_error("start pg_ctl"))?;
        let job = match Job::start(launcher.id()) {
            Ok(job) => job,
            Err(error) => {
                let _ = launcher.kill();
                let _ = launcher.wait();
                return Err(io_error("own the PostgreSQL process tree")(error));
            }
        };
        let child = find_server(&mut launcher, &job, layout)?;
        Ok(Self {
            child,
            spawned_at,
            tree: tree.clone(),
            layout: layout.clone(),
            job: Some(job),
        })
    }
}

fn find_server(
    launcher: &mut std::process::Child,
    job: &Job,
    layout: &Layout,
) -> Result<anima_windows::job::Child, ServerError> {
    let end = Instant::now() + START_LIMIT;
    loop {
        if let Some(status) = launcher.try_wait().map_err(io_error("wait for pg_ctl"))? {
            if !status.success() {
                return Err(ServerError::EndedDuringStart { status });
            }
            let pid = fs::read_to_string(layout.cluster().join("postmaster.pid"))
                .ok()
                .and_then(|text| text.lines().next()?.trim().parse().ok());
            if let Some(pid) = pid {
                return job.child(pid).map_err(io_error("own the database server"));
            }
        }
        if Instant::now() >= end {
            // Dropping the job on error kills the launcher and its descendants.
            return Err(ServerError::NotReady { limit: START_LIMIT });
        }
        std::thread::sleep(super::super::POLL);
    }
}

/// Quote one argument for the Windows C command-line parser used by PostgreSQL.
fn quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        quoted.extend(std::iter::repeat_n(
            '\\',
            if character == '"' {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        quoted.push(character);
        slashes = 0;
    }
    quoted.extend(std::iter::repeat_n('\\', slashes * 2));
    quoted.push('"');
    quoted
}
