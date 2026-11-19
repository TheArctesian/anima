//! The one state of a Compose project (HOST-019).

use super::LISTED;
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{ProjectState, docker},
};

/// The states of the containers that `docker ps` lists for these `(project, state, status)`.
fn marked(rows: &[(&str, &str, &str)]) -> Vec<Option<ProjectState>> {
    let first = LISTED.lines().next().unwrap();
    let text: String = rows
        .iter()
        .map(|(project, state, status)| {
            first
                .replace(
                    r#""project":"dawarich""#,
                    &format!(r#""project":"{project}""#),
                )
                .replace(r#""state":"running""#, &format!(r#""state":"{state}""#))
                .replace("Up 3 hours (healthy)", status)
                + "\n"
        })
        .collect();
    let output = CommandOutput {
        exit_status: Some(0),
        stdout: text.into_bytes(),
        cut: false,
    };
    docker::containers(&output)
        .unwrap()
        .into_iter()
        .map(|container| container.project_state)
        .collect()
}

/// The state of one project with these `(state, status)` containers.
fn project(rows: &[(&str, &str)]) -> ProjectState {
    let rows: Vec<_> = rows
        .iter()
        .map(|(state, status)| ("app", *state, *status))
        .collect();
    let states = marked(&rows);
    assert!(states.iter().all(|state| *state == states[0]), "{states:?}");
    states[0].unwrap()
}

#[test]
fn host_019_one_stopped_container_of_three_makes_the_project_partly_running() {
    let state = project(&[
        ("running", "Up 3 hours"),
        ("running", "Up 3 hours (healthy)"),
        ("exited", "Exited (137) 2 minutes ago"),
    ]);
    assert_eq!(state, ProjectState::PartlyRunning);
    assert_eq!(
        project(&[("running", "Up 1 hour"), ("paused", "Up 1 hour (Paused)")]),
        ProjectState::PartlyRunning
    );
}

#[test]
fn host_019_a_container_that_ended_with_status_0_is_finished_work() {
    let rows = [
        ("running", "Up 3 hours"),
        ("exited", "Exited (0) 3 hours ago"),
    ];
    assert_eq!(project(&rows), ProjectState::Running);
    // With nothing running, the same container is a stopped project.
    assert_eq!(project(&rows[1..]), ProjectState::Stopped);
}

#[test]
fn host_019_the_rules_apply_in_one_order() {
    let running = ("running", "Up 3 hours (healthy)");
    assert_eq!(project(&[running]), ProjectState::Running);
    assert_eq!(
        project(&[running, ("running", "Up 2 minutes (unhealthy)")]),
        ProjectState::Unhealthy
    );
    assert_eq!(
        project(&[("exited", "Exited (1) 1 day ago"), ("created", "Created")]),
        ProjectState::Stopped
    );
    assert_eq!(
        project(&[
            ("paused", "Up 1 hour (Paused)"),
            ("exited", "Exited (1) 1 day ago")
        ]),
        ProjectState::Paused
    );
    // A container that restarts or is dead decides the state before every other rule.
    for failing in [
        ("restarting", "Restarting (1) 5 seconds ago"),
        ("dead", "Dead"),
    ] {
        let rows = [
            running,
            ("exited", "Exited (1) 1 day ago"),
            failing,
            ("hibernating", ""),
        ];
        assert_eq!(project(&rows), ProjectState::Failing);
    }
    assert_eq!(
        project(&[running, ("hibernating", "Up")]),
        ProjectState::Unknown
    );
}

#[test]
fn host_019_each_project_has_its_own_state_and_a_container_with_no_project_has_none() {
    let states = marked(&[
        ("app", "running", "Up 3 hours"),
        ("", "exited", "Exited (1) 1 day ago"),
        ("db", "exited", "Exited (1) 1 day ago"),
        ("app", "running", "Up 3 hours"),
    ]);
    assert_eq!(
        states,
        [
            Some(ProjectState::Running),
            None,
            Some(ProjectState::Stopped),
            Some(ProjectState::Running)
        ]
    );
}
