//! Docker container check: the command and the pure reading of its output (D-3, CHK-012).
//!
//! The hub runs `docker ps -a --no-trunc --format '{{json .}}'` through the SSH link and looks
//! for the configured name among the `Names` of the lines. Nothing of the output is kept: the
//! result holds only fixed words (LOG-007).

use serde::Deserialize;

use super::{
    outcome::{Failure, ProbeResult},
    ssh::readable,
};
use crate::ssh::{CommandOutput, FixedCommand};

/// The command, from static text only: the container name is never part of it.
pub(in crate::checks) fn command() -> FixedCommand {
    FixedCommand::new("docker")
        .arg("ps")
        .arg("-a")
        .arg("--no-trunc")
        .arg("--format")
        .arg("'{{json .}}'")
}

/// The fields of one `docker ps` JSON line the check reads.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Line {
    #[serde(default)]
    names: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    status: String,
}

/// What the container's line says about it.
fn judge(line: &Line) -> ProbeResult {
    if line.state != "running" {
        let class = match line.state.as_str() {
            "exited" => "exited",
            "restarting" => "restarting",
            "paused" => "paused",
            "created" => "created but not started",
            "removing" => "being removed",
            "dead" => "dead",
            _ => "not running",
        };
        return ProbeResult::failed(&Failure::ContainerState(class));
    }
    if line.status.contains("(unhealthy)") {
        ProbeResult::failed(&Failure::ContainerState("running but unhealthy"))
    } else if line.status.contains("(health: starting)") {
        ProbeResult::starting(&Failure::ContainerStarting)
    } else {
        ProbeResult::up_unmeasured()
    }
}

/// The result for the container `name`: the line whose comma-separated `Names` hold it, or
/// not found. A line that is not JSON is skipped, and makes the output unreadable only when
/// no line matched.
pub(in crate::checks) fn read(output: &CommandOutput, name: &str) -> ProbeResult {
    let text = match readable(output) {
        Ok(text) => text,
        Err(failure) => return ProbeResult::failed(&failure),
    };
    let mut skipped = false;
    for text_line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<Line>(text_line) {
            Ok(line) if line.names.split(',').any(|each| each == name) => return judge(&line),
            Ok(_) => {}
            Err(_) => skipped = true,
        }
    }
    ProbeResult::failed(&if skipped {
        Failure::OutputUnreadable
    } else {
        Failure::ContainerNotFound
    })
}
