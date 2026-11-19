//! The details of one container (HOST-017): the command line the device hears and the pure
//! reading of what Docker 29 writes for it.

use chrono::{TimeZone, Utc};
use sqlx::PgPool;

use super::{ID, device, execs, exiting, policy, reach};
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{
        ContainerId, ContainerState, Error, Health, HealthRun, Label, Mount, NetworkAddress,
        Process, detail,
    },
};

/// What `docker container inspect` writes in the hub's format for a running container with a
/// health check, a volume, one network, one label and three environment variables.
const INSPECTED: &str = concat!(
    r#"{"id":"4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a","#,
    r#""name":"/dawarich_app","image":"freikin/dawarich:latest","#,
    r#""imageId":"sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf","#,
    r#""path":"sleep","args":["300"],"workingDir":"/","user":"","#,
    r#""restart":{"MaximumRetryCount":3,"Name":"on-failure"},"state":"running","exitCode":0,"#,
    r#""startedAt":"2026-11-19T07:40:05.279657044Z","finishedAt":"0001-01-01T00:00:00Z","#,
    r#""restarts":2,"check":{"Interval":2000000000,"Test":["CMD-SHELL","true"]},"#,
    r#""health":{"status":"healthy","failing":0,"runs":["#,
    r#"{"end":"2026-11-19T07:40:07.562353086Z","exit":1},"#,
    r#"{"end":"2026-11-19T07:40:09Z","exit":0}]},"#,
    r#""mounts":[{"Destination":"/data","Driver":"local","Mode":"z","Name":"app-data","#,
    r#""Propagation":"","RW":true,"Source":"/var/lib/docker/volumes/app-data/_data","#,
    r#""Type":"volume"}],"#,
    r#""networks":{"bridge":{"Aliases":null,"EndpointID":"47cf","Gateway":"172.17.0.1","#,
    r#""GlobalIPv6Address":"","IPAddress":"172.17.0.2","MacAddress":"6e:c5:bc:92:d3:2a"}},"#,
    r#""labels":{"probe":"1"},"env":["PROBE_SECRET","EMPTY","PATH"]}"#,
);

/// What `docker top` writes for it.
const TOP: &str = "\
UID                 PID                 PPID                C                   STIME               TTY                 TIME                CMD
root                805                 781                 0                   07:40               ?                   00:00:00            sleep 300
999                 806                 805                 0                   07:40               ?                   00:00:01            postgres: writer  process
";

fn output(stdout: &str, exit_status: u32) -> CommandOutput {
    CommandOutput {
        exit_status: Some(exit_status),
        stdout: stdout.as_bytes().to_vec(),
        cut: false,
    }
}

fn container() -> ContainerId {
    ContainerId::parse(ID).unwrap()
}

#[test]
fn host_017_the_details_give_command_restart_policy_health_mounts_networks_and_labels() {
    let read = detail::detail(&output(&format!("{INSPECTED}\n{TOP}"), 0)).unwrap();
    assert_eq!((read.id.as_str(), read.name.as_str()), (ID, "dawarich_app"));
    assert_eq!(read.command, ["sleep", "300"]);
    assert_eq!(read.restart_policy, "on-failure:3");
    assert_eq!((read.state, read.exit_code), (ContainerState::Running, 0));
    assert_eq!(
        read.started_at,
        Utc.with_ymd_and_hms(2026, 11, 19, 7, 40, 5)
            .unwrap()
            .checked_add_signed(chrono::TimeDelta::nanoseconds(279_657_044))
    );
    // Docker writes the year 1 for a container that never ended.
    assert_eq!((read.finished_at, read.restart_count), (None, 2));
    let check = read.health_check.unwrap();
    assert_eq!(check.test, ["CMD-SHELL", "true"]);
    assert_eq!(check.interval_seconds, Some(2));
    assert_eq!((read.health, read.failing_streak), (Health::Healthy, 0));
    assert_eq!(
        read.last_health_run,
        Some(HealthRun {
            ended_at: Some(Utc.with_ymd_and_hms(2026, 11, 19, 7, 40, 9).unwrap()),
            exit_code: 0,
        })
    );
    let mount = Mount {
        kind: "volume".to_owned(),
        name: Some("app-data".to_owned()),
        source: "/var/lib/docker/volumes/app-data/_data".to_owned(),
        destination: "/data".to_owned(),
        read_only: false,
    };
    assert_eq!(read.mounts, [mount]);
    let address = NetworkAddress {
        network: "bridge".to_owned(),
        ipv4: Some("172.17.0.2".to_owned()),
        ipv6: None,
        gateway: Some("172.17.0.1".to_owned()),
        mac_address: Some("6e:c5:bc:92:d3:2a".to_owned()),
    };
    assert_eq!(read.networks, [address]);
    let label = Label {
        name: "probe".to_owned(),
        value: "1".to_owned(),
    };
    assert_eq!(read.labels, [label]);
    assert_eq!(read.environment_names, ["PROBE_SECRET", "EMPTY", "PATH"]);
}

#[test]
fn host_017_the_processes_are_the_rows_of_docker_top() {
    let read = detail::detail(&output(&format!("{INSPECTED}\n{TOP}"), 0)).unwrap();
    let process = |pid, user: &str, command: &str| Process {
        pid,
        user: Some(user.to_owned()),
        command: command.to_owned(),
    };
    assert_eq!(
        read.processes,
        [
            process(805, "root", "sleep 300"),
            process(806, "999", "postgres: writer  process"),
        ]
    );
}

/// What Docker 29 writes for a container that was stopped: the last health status stays,
/// and the network has no address.
const STOPPED: &str = concat!(
    r#"{"id":"4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a","#,
    r#""name":"/old-db","image":"postgres:17","imageId":"sha256:f59a","path":"postgres","#,
    r#""args":null,"workingDir":"","user":"","restart":{"MaximumRetryCount":0,"Name":"no"},"#,
    r#""state":"exited","exitCode":137,"startedAt":"2026-11-19T07:40:05Z","#,
    r#""finishedAt":"2026-11-19T07:45:00Z","restarts":0,"check":{"Test":["CMD","pg_isready"]},"#,
    r#""health":{"status":"unhealthy","failing":3,"runs":[]},"mounts":null,"#,
    r#""networks":{"bridge":{"IPAddress":"","Gateway":""},"host":null},"labels":null,"env":null}"#,
);

#[test]
fn host_017_a_container_that_does_not_run_has_its_details_and_no_process() {
    let text = format!("{STOPPED}\nError response from daemon: container {ID} is not running\n");
    // `docker top` failed, so the command line did not exit 0.
    let read = detail::detail(&output(&text, 1)).unwrap();
    assert_eq!((read.state, read.exit_code), (ContainerState::Exited, 137));
    assert_eq!(
        (read.command.as_slice(), read.restart_policy.as_str()),
        (["postgres".to_owned()].as_slice(), "no")
    );
    // The last status of the health check is not the health of a container that does not run.
    assert_eq!(
        (read.health, read.failing_streak, read.last_health_run),
        (Health::None, 3, None)
    );
    // The default interval of Docker is not written.
    assert_eq!(read.health_check.unwrap().interval_seconds, None);
    assert_eq!(read.networks.len(), 2);
    assert_eq!(
        (&read.networks[0].ipv4, &read.networks[1].ipv4),
        (&None, &None)
    );
    assert_eq!(
        (read.labels, read.mounts, read.processes),
        (vec![], vec![], vec![])
    );
    assert_eq!(read.environment_names, [] as [String; 0]);
}

#[test]
fn host_017_a_warning_of_docker_before_either_part_hides_nothing() {
    let warning = "WARNING: Error loading config file: /root/.docker/config.json\n";
    let text = format!("{warning}{INSPECTED}\n{warning}{TOP}");
    let read = detail::detail(&output(&text, 0)).unwrap();
    assert_eq!(
        (read.name.as_str(), read.processes.len()),
        ("dawarich_app", 2)
    );
    // A line of the format that cannot be read is not a Docker that failed.
    let broken = format!(
        "{}\nError response from daemon: not running\n",
        &INSPECTED[..60]
    );
    assert!(matches!(
        detail::detail(&output(&broken, 1)),
        Err(Error::Unreadable)
    ));
}

#[test]
fn host_017_an_unknown_container_and_an_answer_that_is_not_the_format_are_told_apart() {
    let unknown = format!("Error response from daemon: No such container: {ID}\n");
    assert!(matches!(
        detail::detail(&output(&unknown, 1)),
        Err(Error::NoContainer)
    ));
    assert!(matches!(
        detail::detail(&output("docker: not found\n", 127)),
        Err(Error::NoDocker)
    ));
    assert!(matches!(
        detail::detail(&output("permission denied\n", 1)),
        Err(Error::DockerFailed)
    ));
    assert!(matches!(
        detail::detail(&output("{}\n", 0)),
        Err(Error::Unreadable)
    ));
    let cut = CommandOutput {
        cut: true,
        ..output(INSPECTED, 0)
    };
    assert!(matches!(detail::detail(&cut), Err(Error::Unreadable)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_017_the_command_asks_for_the_names_of_the_environment_and_never_a_value(
    pool: PgPool,
) {
    let bench = device(&pool, exiting(&format!("{INSPECTED}\n{TOP}"), 0)).await;
    let policy = policy();
    let now = crate::fleet::ssh::tests::fixture::now();
    let read =
        crate::work::containers::detail(reach(&bench, &policy), bench.device.id, &container(), now)
            .await
            .unwrap();
    assert_eq!(read.processes.len(), 2);
    let heard = execs(&bench);
    assert_eq!(heard.len(), 1);
    // The whole line: the only use of the environment is the text before the first `=` of
    // each entry, and the output of a health check is not asked for.
    let format = concat!(
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
    assert_eq!(
        heard[0],
        format!("docker container inspect --format {format} {ID} 2>&1 && docker top {ID} 2>&1")
    );
    // The value shown for a logged read holds no text of the device.
    assert_eq!(
        format!("{read:?}"),
        format!("ContainerDetail {{ id: {ID:?}, .. }}")
    );
}
