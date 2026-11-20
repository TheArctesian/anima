//! `anima containers projects <device>`: the Compose projects of a device, each with its one
//! state and where Compose found it (HOST-019, R-211). Anima reads the paths and changes no
//! file.

use std::collections::BTreeMap;

use serde::Serialize;

use super::documents::container_list::ContainerListContainers;
use super::list::fetch;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct ProjectsArgs {
    /// The device's ID or its exact name.
    device: String,
}

/// One Compose project of a device, made from its containers.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    /// The state the hub made (K1 `ComposeProjectState`), as its name.
    pub state: Option<String>,
    pub containers: usize,
    pub running: usize,
    /// The folder from which Compose started the project; `None` when Docker has no such label.
    pub working_dir: Option<String>,
    /// The Compose files, as paths on the device.
    pub files: Vec<String>,
}

/// The projects of these containers, by name. A container with no project is in none.
#[must_use]
pub fn projects(containers: &[ContainerListContainers]) -> Vec<Project> {
    let mut found: BTreeMap<&str, Project> = BTreeMap::new();
    for container in containers {
        let Some(name) = container.compose_project.as_deref() else {
            continue;
        };
        let project = found.entry(name).or_insert_with(|| Project {
            name: name.to_owned(),
            state: container.compose_project_state.as_ref().map(enum_name),
            containers: 0,
            running: 0,
            working_dir: None,
            files: Vec::new(),
        });
        project.containers += 1;
        project.running += usize::from(enum_name(&container.state) == "RUNNING");
        // Each container of a project has the same two labels; an old container can have none.
        if project.working_dir.is_none() {
            project
                .working_dir
                .clone_from(&container.compose_working_dir);
        }
        if project.files.is_empty() {
            project.files.clone_from(&container.compose_files);
        }
    }
    found.into_values().collect()
}

impl ProjectsArgs {
    /// Prints a row for each project or, with `--json`, the list of [`Project`].
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal when the device has no
    /// SSH credential, has no Docker or does not answer.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let containers = fetch(&invocation.hub()?, &self.device).await?;
        invocation.output.data(&projects(&containers), table)
    }
}

fn table(found: &Vec<Project>) -> Table {
    let mut table = Table::new(vec!["PROJECT", "STATE", "RUNNING", "FOLDER", "FILES"]);
    for row in found {
        let state = row.state.as_deref().unwrap_or("-");
        table.row(vec![
            row.name.as_str().into(),
            Cell::new(&state.to_lowercase().replace('_', " ")),
            format!("{} of {}", row.running, row.containers).into(),
            row.working_dir.as_deref().unwrap_or("-").into(),
            Cell::new(&row.files.join(", ")),
        ]);
    }
    table
}
