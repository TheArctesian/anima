//! The one state of a Compose project, made from the states of its containers (HOST-019).
//!
//! One rule, in this order; the first that applies gives the state:
//!
//! 1. A container restarts, or is dead: `Failing`.
//! 2. Some containers run and some do not: `PartlyRunning`. A container that ended with exit
//!    status 0 is finished work (a migration, a set-up step) and does not count here.
//! 3. Every container that counts runs, and one fails its health check: `Unhealthy`.
//! 4. Every container that counts runs: `Running`.
//! 5. No container runs and one is paused: `Paused`.
//! 6. No container runs: `Stopped`.
//!
//! A state the hub does not know makes the project `Unknown`, unless rule 1 applies.

use std::collections::BTreeMap;

use super::model::{Container, ContainerState, Health, ProjectState};

/// What one container adds to the state of its project.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Failing,
    Running {
        unhealthy: bool,
    },
    Paused,
    /// Ended with exit status 0.
    Finished,
    Stopped,
    Unknown,
}

fn part(container: &Container) -> Part {
    match container.state {
        ContainerState::Restarting | ContainerState::Dead => Part::Failing,
        ContainerState::Running => Part::Running {
            unhealthy: container.health == Health::Unhealthy,
        },
        ContainerState::Paused => Part::Paused,
        ContainerState::Exited if container.status.starts_with("Exited (0)") => Part::Finished,
        ContainerState::Exited | ContainerState::Created | ContainerState::Removing => {
            Part::Stopped
        }
        ContainerState::Unknown => Part::Unknown,
    }
}

/// The state of a project that has these containers.
fn state(parts: &[Part]) -> ProjectState {
    let has = |wanted: fn(&Part) -> bool| parts.iter().any(wanted);
    let runs = |part: &Part| matches!(part, Part::Running { .. });
    if has(|part| *part == Part::Failing) {
        ProjectState::Failing
    } else if has(|part| *part == Part::Unknown) {
        ProjectState::Unknown
    } else if !has(runs) {
        if has(|part| *part == Part::Paused) {
            ProjectState::Paused
        } else {
            ProjectState::Stopped
        }
    } else if has(|part| matches!(part, Part::Paused | Part::Stopped)) {
        ProjectState::PartlyRunning
    } else if has(|part| matches!(part, Part::Running { unhealthy: true })) {
        ProjectState::Unhealthy
    } else {
        ProjectState::Running
    }
}

/// Writes on each container of a Compose project the state of that project.
pub(super) fn mark(containers: &mut [Container]) {
    let mut parts: BTreeMap<String, Vec<Part>> = BTreeMap::new();
    for container in containers.iter() {
        if let Some(project) = &container.compose_project {
            parts
                .entry(project.clone())
                .or_default()
                .push(part(container));
        }
    }
    for container in containers {
        container.project_state = container
            .compose_project
            .as_ref()
            .and_then(|project| parts.get(project))
            .map(|parts| state(parts));
    }
}
