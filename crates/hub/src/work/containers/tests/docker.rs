//! The pure reading of `docker ps` and `docker logs` output, and the two checked values
//! (HOST-001).

use chrono::{TimeZone, Utc};

use super::{ID, LISTED};
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{
        Container, ContainerId, ContainerState, Error, Health, LineCount, ProjectState, docker,
    },
};

fn output(stdout: &str, exit_status: Option<u32>) -> CommandOutput {
    CommandOutput {
        exit_status,
        stdout: stdout.as_bytes().to_vec(),
        cut: false,
    }
}

#[test]
fn host_001_the_list_gives_name_image_state_status_ports_and_compose_project() {
    let containers = docker::containers(&output(LISTED, Some(0))).unwrap();
    let app = Container {
        id: ID.to_owned(),
        name: "dawarich_app".to_owned(),
        image: "freikin/dawarich:latest".to_owned(),
        state: ContainerState::Running,
        status: "Up 3 hours (healthy)".to_owned(),
        health: Health::Healthy,
        ports: vec![
            "0.0.0.0:3000->3000/tcp".to_owned(),
            "[::]:3000->3000/tcp".to_owned(),
        ],
        created_at: Some(Utc.with_ymd_and_hms(2026, 11, 18, 18, 0, 0).unwrap()),
        compose_project: Some("dawarich".to_owned()),
        compose_service: Some("app".to_owned()),
        compose_working_dir: Some("/srv/dawarich".to_owned()),
        compose_files: vec![
            "/srv/dawarich/compose.yaml".to_owned(),
            "/srv/dawarich/override.yaml".to_owned(),
        ],
        project_state: Some(ProjectState::Running),
    };
    assert_eq!(containers[0], app);
    let db = &containers[1];
    // The name a link of another container gives (`web/db`) is not the container's own.
    assert_eq!(db.name, "old-db");
    assert_eq!(
        (db.state, db.health, db.created_at),
        (ContainerState::Exited, Health::None, None)
    );
    assert_eq!((&db.ports, &db.compose_project), (&vec![], &None));
    // A container of no project, in the answer of a list that has no such label.
    assert_eq!(
        (&db.compose_working_dir, &db.compose_files),
        (&None, &vec![])
    );
    assert_eq!(containers.len(), 2);
}

#[test]
fn host_001_a_device_with_no_container_has_an_empty_list() {
    assert_eq!(docker::containers(&output("", Some(0))).unwrap(), []);
}

#[test]
fn host_001_the_status_text_gives_the_health_and_an_unknown_state_is_unknown() {
    let line = |state: &str, status: &str| {
        let text = LISTED
            .lines()
            .next()
            .unwrap()
            .replace(r#""state":"running""#, &format!(r#""state":"{state}""#))
            .replace("Up 3 hours (healthy)", status);
        let read = docker::containers(&output(&text, Some(0)))
            .unwrap()
            .remove(0);
        (read.state, read.health)
    };
    assert_eq!(
        line("running", "Up 2 minutes (unhealthy)"),
        (ContainerState::Running, Health::Unhealthy)
    );
    assert_eq!(
        line("restarting", "Up 1 second (health: starting)"),
        (ContainerState::Restarting, Health::Starting)
    );
    assert_eq!(
        line("hibernating", "Up 1 hour"),
        (ContainerState::Unknown, Health::None)
    );
}

#[test]
fn host_010_a_shell_with_no_docker_and_a_docker_that_fails_are_told_apart() {
    assert!(matches!(
        docker::containers(&output("", Some(127))),
        Err(Error::NoDocker)
    ));
    assert!(matches!(
        docker::containers(&output("", Some(1))),
        Err(Error::DockerFailed)
    ));
    assert!(matches!(
        docker::containers(&output("", None)),
        Err(Error::DockerFailed)
    ));
}

#[test]
fn host_001_a_list_that_was_cut_or_is_not_the_format_is_unreadable_not_partial() {
    let cut = CommandOutput {
        exit_status: None,
        stdout: LISTED.as_bytes().to_vec(),
        cut: true,
    };
    assert!(matches!(docker::containers(&cut), Err(Error::Unreadable)));
    let stray = format!("{LISTED}WARNING: something\n");
    assert!(matches!(
        docker::containers(&output(&stray, Some(0))),
        Err(Error::Unreadable)
    ));
}

#[test]
fn host_001_log_lines_are_oldest_first_with_the_time_docker_wrote() {
    let text = "2026-11-18T18:00:00.123456789Z listening on :3000\n\
                2026-11-18T18:00:01Z   indented text stays\r\n\
                no time on this line\n";
    let log = docker::log(&output(text, Some(0))).unwrap();
    let read: Vec<(Option<String>, &str)> = log
        .lines
        .iter()
        .map(|line| (line.at.map(|at| at.to_rfc3339()), line.text.as_str()))
        .collect();
    assert_eq!(
        read,
        [
            (
                Some("2026-11-18T18:00:00.123456789+00:00".to_owned()),
                "listening on :3000"
            ),
            (
                Some("2026-11-18T18:00:01+00:00".to_owned()),
                "  indented text stays"
            ),
            (None, "no time on this line"),
        ]
    );
    assert!(!log.cut);
}

#[test]
fn host_001_a_log_cut_at_the_cap_keeps_whole_lines_and_says_so() {
    let cut = CommandOutput {
        exit_status: None,
        stdout: b"2026-11-18T18:00:00Z first\n2026-11-18T18:00:01Z sec".to_vec(),
        cut: true,
    };
    let log = docker::log(&cut).unwrap();
    assert!(log.cut);
    assert_eq!(log.lines.len(), 1);
    assert_eq!(log.lines[0].text, "first");
}

#[test]
fn host_001_bytes_that_are_not_utf_8_do_not_lose_the_log() {
    let output = CommandOutput {
        exit_status: Some(0),
        stdout: b"2026-11-18T18:00:00Z caf\xe9\n".to_vec(),
        cut: false,
    };
    assert_eq!(docker::log(&output).unwrap().lines[0].text, "caf\u{fffd}");
}

#[test]
fn host_001_a_log_of_no_such_container_is_told_apart_from_a_docker_that_fails() {
    let unknown = "Error response from daemon: No such container: 4f5c1d2e3a4b\n";
    assert!(matches!(
        docker::log(&output(unknown, Some(1))),
        Err(Error::NoContainer)
    ));
    assert!(matches!(
        docker::log(&output("permission denied\n", Some(1))),
        Err(Error::DockerFailed)
    ));
    // A line of the container that holds the words does not make the answer "not found".
    let quoted = "2026-11-18T18:00:00Z app: No such container: x\nError: the log driver failed\n";
    assert!(matches!(
        docker::log(&output(quoted, Some(1))),
        Err(Error::DockerFailed)
    ));
    assert!(matches!(
        docker::log(&output("sh: docker: not found\n", Some(127))),
        Err(Error::NoDocker)
    ));
}

#[test]
fn host_001_a_container_identifier_is_12_to_64_lower_case_hex_digits() {
    for accepted in [&ID[..12], &ID[..13], ID] {
        assert!(ContainerId::parse(accepted).is_ok(), "{accepted}");
    }
    let long = format!("{ID}a");
    for refused in [
        "",
        &ID[..11],
        long.as_str(),
        "4F5C1D2E3A4B",
        "dawarich_app",
        "4f5c1d2e3a4b; reboot",
        "4f5c1d2e3a4b\n",
        "$(reboot)aaaa",
    ] {
        assert!(
            matches!(ContainerId::parse(refused), Err(Error::InvalidContainerId)),
            "{refused:?}"
        );
    }
}

#[test]
fn host_001_the_number_of_lines_is_1_to_1000_and_200_when_not_given() {
    assert_eq!(LineCount::new(None).unwrap().get(), 200);
    assert_eq!(LineCount::new(Some(1)).unwrap().get(), 1);
    assert_eq!(LineCount::new(Some(1000)).unwrap().get(), 1000);
    for refused in [0, -1, 1001, i32::MAX] {
        assert!(matches!(
            LineCount::new(Some(refused)),
            Err(Error::InvalidLineCount)
        ));
    }
}

#[test]
fn log_007_the_debug_form_of_a_log_holds_no_line() {
    let log = docker::log(&output("2026-11-18T18:00:00Z API_TOKEN=hunter2\n", Some(0))).unwrap();
    let shown = format!("{log:?}");
    assert!(!shown.contains("hunter2"), "{shown}");
}
