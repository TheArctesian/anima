//! `anima project group`: the operator's own groups of projects (PRO-059 to PRO-063). A group
//! is named by its name or its ID; projects by their slugs.

pub mod documents;
pub mod lookup;

use serde::Serialize;
use serde_json::Value;

use self::documents::{
    ProjectGroupCreate, ProjectGroupDelete, ProjectGroupRename, ProjectsGroupSet,
    project_group_create, project_group_delete, project_group_rename, projects_group_set,
};
use self::lookup::{group_id, groups, project_ids};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum GroupCommand {
    /// List the groups, with the number of projects in each.
    List,
    /// Make a group, with the projects that you name in it.
    Create {
        /// 1 to 60 characters.
        name: String,
        /// The slugs of the projects to put in it; a project in a different group moves.
        projects: Vec<String>,
    },
    /// Give a group a different name.
    Rename {
        /// The group's name or ID.
        group: String,
        /// The new name.
        name: String,
    },
    /// Delete a group. Its projects stay, with no group; none is left out.
    Delete {
        /// The group's name or ID.
        group: String,
    },
    /// Put projects in a group; a project in a different group moves.
    Add {
        /// The group's name or ID.
        group: String,
        /// The slugs of the projects.
        #[arg(required = true)]
        projects: Vec<String>,
    },
    /// Take projects out of their group.
    Remove {
        /// The slugs of the projects.
        #[arg(required = true)]
        projects: Vec<String>,
    },
}

/// Groups as rows: name, projects, ID.
fn table<T: Serialize>(value: &T) -> Table {
    let value = serde_json::to_value(value).unwrap_or_default();
    let rows = match value {
        Value::Array(rows) => rows,
        one => vec![one],
    };
    let mut table = Table::new(vec!["NAME", "PROJECTS", "ID"]);
    for group in &rows {
        table.row(vec![
            group["name"].as_str().unwrap_or("-").into(),
            group["projectCount"].to_string().into(),
            group["id"].as_str().unwrap_or("-").into(),
        ]);
    }
    table
}

/// The IDs a change returns, one for each line.
fn ids<T: Serialize>(value: &T) -> Table {
    let value = serde_json::to_value(value).unwrap_or_default();
    let mut table = Table::new(vec!["ID"]);
    for id in value.as_array().map_or(&[][..], Vec::as_slice) {
        table.row(vec![id.as_str().unwrap_or("-").into()]);
    }
    table
}

impl GroupCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown group or project; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        match self {
            Self::List => invocation.output.data(&groups(&hub).await?, table),
            Self::Create { name, projects } => {
                let projects = project_ids(&hub, &projects).await?;
                let variables = project_group_create::Variables { name, projects };
                let made = hub.run::<ProjectGroupCreate>(variables).await?;
                invocation.output.data(&made.create_project_group, table)
            }
            Self::Rename { group, name } => {
                let id = group_id(&hub, &group).await?;
                let variables = project_group_rename::Variables { id, name };
                let renamed = hub.run::<ProjectGroupRename>(variables).await?;
                invocation.output.data(&renamed.rename_project_group, table)
            }
            Self::Delete { group } => {
                let id = group_id(&hub, &group).await?;
                let variables = project_group_delete::Variables { id };
                let deleted = hub.run::<ProjectGroupDelete>(variables).await?;
                invocation
                    .output
                    .data(&vec![deleted.delete_project_group], ids)
            }
            Self::Add { group, projects } => {
                let group = Some(group_id(&hub, &group).await?);
                let projects = project_ids(&hub, &projects).await?;
                let variables = projects_group_set::Variables { projects, group };
                let moved = hub.run::<ProjectsGroupSet>(variables).await?;
                invocation.output.data(&moved.set_projects_group, ids)
            }
            Self::Remove { projects } => {
                let projects = project_ids(&hub, &projects).await?;
                let variables = projects_group_set::Variables {
                    projects,
                    group: None,
                };
                let moved = hub.run::<ProjectsGroupSet>(variables).await?;
                invocation.output.data(&moved.set_projects_group, ids)
            }
        }
    }
}
