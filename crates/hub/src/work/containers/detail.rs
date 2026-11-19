//! The details of one container, read-only (HOST-017): the `docker` command line and the pure
//! reading of its output.
//!
//! The format asks Docker for named values only. For the environment it asks for the text
//! before the first `=` of each entry, so no value of an environment variable leaves the
//! device (LOG-007). The output of a health check is not asked for. One command line reads
//! the container and then its processes; a container that does not run has no process.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{
    Error,
    docker::{failure, says_no_container, state},
    model::{
        ContainerDetail, ContainerId, Health, HealthCheck, HealthRun, Label, Mount, NetworkAddress,
        Process,
    },
};
use crate::fleet::ssh::{CommandOutput, FixedCommand};

/// One JSON object for the container, made by Docker's template from static text.
const DETAIL_FORMAT: &str = concat!(
    r#"'{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"#,
    r#""imageId":{{json .Image}},"path":{{json .Path}},"args":{{json .Args}},"#,
    r#""workingDir":{{json .Config.WorkingDir}},"user":{{json .Config.User}},"#,
    r#""restart":{{json .HostConfig.RestartPolicy}},"state":{{json .State.Status}},"#,
    r#""exitCode":{{json .State.ExitCode}},"startedAt":{{json .State.StartedAt}},"#,
    r#""finishedAt":{{json .State.FinishedAt}},"restarts":{{json .RestartCount}},"#,
    r#""check":{{json .Config.Healthcheck}},"#,
    r#""health":{{with .State.Health}}{"status":{{json .Status}},"#,
    r#""failing":{{json .FailingStreak}},"runs":[{{range $i, $run := .Log}}{{if $i}},{{end}}"#,
    r#"{"end":{{json $run.End}},"exit":{{json $run.ExitCode}}}{{end}}]}{{else}}null{{end}},"#,
    r#""mounts":{{json .Mounts}},"networks":{{json .NetworkSettings.Networks}},"#,
    r#""labels":{{json .Config.Labels}},"#,
    r#""env":[{{range $i, $e := .Config.Env}}{{if $i}},{{end}}"#,
    r#"{{json (index (split $e "=") 0)}}{{end}}]}'"#,
);

/// `docker container inspect` for the named values of one container, then `docker top` for
/// its processes. The second command runs only when the first succeeded.
pub(super) fn command(container: &ContainerId) -> FixedCommand {
    FixedCommand::new("docker")
        .arg("container")
        .arg("inspect")
        .arg("--format")
        .arg(DETAIL_FORMAT)
        .hex(container.digits())
        .arg("2>&1")
        .arg("&&")
        .arg("docker")
        .arg("top")
        .hex(container.digits())
        .arg("2>&1")
}

/// The container, as [`DETAIL_FORMAT`] writes it. Docker writes `null` for a list or a map
/// that is empty.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inspected {
    id: String,
    name: String,
    image: String,
    image_id: String,
    path: String,
    args: Option<Vec<String>>,
    working_dir: String,
    user: String,
    restart: Restart,
    state: String,
    exit_code: i32,
    started_at: String,
    finished_at: String,
    restarts: u32,
    check: Option<Check>,
    health: Option<HealthRead>,
    mounts: Option<Vec<MountRead>>,
    networks: Option<BTreeMap<String, Endpoint>>,
    labels: Option<BTreeMap<String, String>>,
    env: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Restart {
    name: String,
    maximum_retry_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Check {
    test: Option<Vec<String>>,
    /// Nanoseconds.
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct HealthRead {
    status: String,
    failing: u32,
    runs: Vec<RunRead>,
}

#[derive(Deserialize)]
struct RunRead {
    end: String,
    exit: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MountRead {
    #[serde(rename = "Type")]
    kind: String,
    name: Option<String>,
    source: String,
    destination: String,
    #[serde(rename = "RW")]
    writable: bool,
}

#[derive(Deserialize)]
struct Endpoint {
    #[serde(rename = "IPAddress")]
    ipv4: String,
    #[serde(rename = "GlobalIPv6Address")]
    ipv6: String,
    #[serde(rename = "Gateway")]
    gateway: String,
    #[serde(rename = "MacAddress")]
    mac: String,
}

/// The details the command line wrote: the container on the first line, then the table of
/// `docker top`, or Docker's error when the container does not run.
///
/// # Errors
/// `NoContainer` when Docker says it knows no such container; `Unreadable` when the output
/// was cut or the first line is not what the format writes; `NoDocker` or `DockerFailed` when
/// the first command did not succeed for another reason.
pub(super) fn detail(output: &CommandOutput) -> Result<ContainerDetail, Error> {
    if output.cut {
        return Err(Error::Unreadable);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (first, rest) = text.split_once('\n').unwrap_or((&text, ""));
    let Ok(read) = serde_json::from_str::<Inspected>(first) else {
        return Err(match output.exit_status {
            Some(0) => Error::Unreadable,
            _ if says_no_container(&text) => Error::NoContainer,
            _ => failure(output),
        });
    };
    Ok(container(read, processes(rest)))
}

fn container(read: Inspected, processes: Vec<Process>) -> ContainerDetail {
    let mut command = vec![read.path];
    command.extend(read.args.unwrap_or_default());
    let health = read.health.as_ref();
    ContainerDetail {
        id: read.id,
        name: read.name.trim_start_matches('/').to_owned(),
        image: read.image,
        image_id: read.image_id,
        command,
        working_dir: read.working_dir,
        user: read.user,
        restart_policy: restart_policy(&read.restart),
        state: state(&read.state),
        exit_code: read.exit_code,
        started_at: instant(&read.started_at),
        finished_at: instant(&read.finished_at),
        restart_count: read.restarts,
        health_check: read.check.and_then(health_check),
        health: health.map_or(Health::None, |health| health_of(&health.status)),
        failing_streak: health.map_or(0, |health| health.failing),
        last_health_run: health
            .and_then(|health| health.runs.last())
            .map(|run| HealthRun {
                ended_at: instant(&run.end),
                exit_code: run.exit,
            }),
        mounts: read
            .mounts
            .unwrap_or_default()
            .into_iter()
            .map(mount)
            .collect(),
        networks: read
            .networks
            .unwrap_or_default()
            .into_iter()
            .map(|(name, endpoint)| address(name, endpoint))
            .collect(),
        labels: read
            .labels
            .unwrap_or_default()
            .into_iter()
            .map(|(name, value)| Label { name, value })
            .collect(),
        environment_names: read.env,
        processes,
    }
}

/// `no`, `always`, `unless-stopped`, or `on-failure` with its limit, as `docker run` takes it.
fn restart_policy(restart: &Restart) -> String {
    match (restart.name.as_str(), restart.maximum_retry_count) {
        ("", _) => "no".to_owned(),
        (name, 0) => name.to_owned(),
        (name, limit) => format!("{name}:{limit}"),
    }
}

/// The health check of the image or of the container; `None` when it is turned off.
fn health_check(check: Check) -> Option<HealthCheck> {
    let test = check.test.filter(|test| !test.is_empty())?;
    if test.first().map(String::as_str) == Some("NONE") {
        return None;
    }
    Some(HealthCheck {
        test,
        interval_seconds: check
            .interval
            .and_then(|nanoseconds| u32::try_from(nanoseconds / 1_000_000_000).ok()),
    })
}

fn health_of(status: &str) -> Health {
    match status {
        "healthy" => Health::Healthy,
        "unhealthy" => Health::Unhealthy,
        "starting" => Health::Starting,
        _ => Health::None,
    }
}

/// A time of Docker; `None` for the year 1, which Docker writes for "never", and for text
/// that is not a time.
fn instant(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.with_timezone(&Utc))
        .filter(|at| at.timestamp() > 0)
}

fn mount(read: MountRead) -> Mount {
    Mount {
        kind: read.kind,
        name: read.name.filter(|name| !name.is_empty()),
        source: read.source,
        destination: read.destination,
        read_only: !read.writable,
    }
}

fn address(network: String, endpoint: Endpoint) -> NetworkAddress {
    let filled = |text: String| Some(text).filter(|text| !text.is_empty());
    NetworkAddress {
        network,
        ipv4: filled(endpoint.ipv4),
        ipv6: filled(endpoint.ipv6),
        gateway: filled(endpoint.gateway),
        mac_address: filled(endpoint.mac),
    }
}

/// The rows of the table of `docker top`: the columns `PID`, `UID` or `USER`, and the last
/// column, which is the command. Any other text gives no process.
fn processes(table: &str) -> Vec<Process> {
    let mut lines = table.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns: Vec<&str> = header.split_whitespace().collect();
    let Some(pid) = columns.iter().position(|column| *column == "PID") else {
        return Vec::new();
    };
    let user = columns
        .iter()
        .position(|column| matches!(*column, "UID" | "USER"));
    lines
        .filter_map(|line| process(line, columns.len(), (pid, user)))
        .collect()
}

/// One row of `columns` columns; the last column takes the rest of the line.
fn process(line: &str, columns: usize, (pid, user): (usize, Option<usize>)) -> Option<Process> {
    let mut rest = line.trim_start();
    let mut words = Vec::with_capacity(columns);
    for _ in 1..columns {
        let (word, after) = rest.split_once(char::is_whitespace)?;
        words.push(word);
        rest = after.trim_start();
    }
    Some(Process {
        pid: words.get(pid)?.parse().ok()?,
        user: user
            .and_then(|user| words.get(user))
            .map(|user| (*user).to_owned()),
        command: rest.to_owned(),
    })
}
