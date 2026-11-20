//! The project group tools of the `projects` set (PRO-059 to PRO-063). A group is the
//! operator's own label; its name is his text, not text from GitHub.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::failure::Failure;
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::projects::group::documents::{
    ProjectGroupCreate, ProjectGroupDelete, ProjectGroupRename, ProjectsGroupSet,
    project_group_create, project_group_delete, project_group_rename, projects_group_set,
};
use crate::projects::group::lookup::{group_id, groups, project_ids};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupTool {
    List,
    Create,
    Rename,
    Delete,
    SetProjects,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInput {
    /// 1 to 60 characters; no two groups share a name, whatever the case of the letters.
    name: String,
    /// The slugs of the projects to put in it; a project in a different group moves.
    #[serde(default)]
    projects: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameInput {
    /// The group's name or ID (see `anima_project_groups_list`).
    group: String,
    /// The new name.
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    /// The group's name or ID.
    group: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetInput {
    /// The slugs of the projects, at most 200.
    projects: Vec<String>,
    /// The group's name or ID; leave it out to take the projects out of their group.
    group: Option<String>,
}

/// A missing group or project as a refusal the agent can act on.
fn lookup(failure: Failure) -> ToolError {
    match failure {
        Failure::NotFound { message, .. } => ToolError::Refused(format!(
            "{message} Call anima_project_groups_list or anima_projects_search."
        )),
        other => ToolError::Failed(other),
    }
}

impl GroupTool {
    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_project_groups_list",
                Access::Read,
                Effect::Reads,
                "Lists the operator's groups of projects in name order: id, name, the number of \
                 projects that are not left out, and when it was made. A project is in at most \
                 one group.",
            )
            .returns(&[("groups", "array", "The groups.")]),
            Self::Create => Definition::new::<CreateInput>(
                "anima_project_group_create",
                Access::Write,
                Effect::Sets,
                "Makes a group of projects with a name and puts the named projects in it. A \
                 project that is in a different group moves. Returns the group.",
            )
            .returns(&[("group", "object", "The new group.")]),
            Self::Rename => Definition::new::<RenameInput>(
                "anima_project_group_rename",
                Access::Write,
                Effect::Sets,
                "Gives a group of projects a different name. Returns the group.",
            )
            .returns(&[("group", "object", "The group after the change.")]),
            Self::Delete => Definition::new::<DeleteInput>(
                "anima_project_group_delete",
                Access::Write,
                Effect::Sets,
                "Deletes a group of projects. Its projects stay, with no group, and none is \
                 left out. Returns the id.",
            )
            .returns(&[("id", "string", "The id of the deleted group.")]),
            Self::SetProjects => Definition::new::<SetInput>(
                "anima_projects_set_group",
                Access::Write,
                Effect::Sets,
                "Puts projects in a group, or with no group takes them out of their group. \
                 Returns the ids of the projects that moved.",
            )
            .returns(&[("moved", "array", "The ids of the projects that moved.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, an unknown group or project, or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => {
                let _: ListInput = parse(arguments)?;
                Ok(Value::Object(object([(
                    "groups",
                    json(&groups(hub).await?),
                )])))
            }
            Self::Create => {
                let input: CreateInput = parse(arguments)?;
                let projects = project_ids(hub, &input.projects).await.map_err(lookup)?;
                let variables = project_group_create::Variables {
                    name: input.name,
                    projects,
                };
                let made = hub.run::<ProjectGroupCreate>(variables).await?;
                Ok(Value::Object(object([(
                    "group",
                    json(&made.create_project_group),
                )])))
            }
            Self::Rename => {
                let input: RenameInput = parse(arguments)?;
                let id = group_id(hub, &input.group).await.map_err(lookup)?;
                let variables = project_group_rename::Variables {
                    id,
                    name: input.name,
                };
                let renamed = hub.run::<ProjectGroupRename>(variables).await?;
                Ok(Value::Object(object([(
                    "group",
                    json(&renamed.rename_project_group),
                )])))
            }
            Self::Delete => {
                let input: DeleteInput = parse(arguments)?;
                let id = group_id(hub, &input.group).await.map_err(lookup)?;
                let variables = project_group_delete::Variables { id };
                let deleted = hub.run::<ProjectGroupDelete>(variables).await?;
                Ok(Value::Object(object([(
                    "id",
                    json(&deleted.delete_project_group),
                )])))
            }
            Self::SetProjects => set_projects(hub, arguments).await,
        }
    }
}

async fn set_projects(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: SetInput = parse(arguments)?;
    let group = match &input.group {
        Some(group) => Some(group_id(hub, group).await.map_err(lookup)?),
        None => None,
    };
    let projects = project_ids(hub, &input.projects).await.map_err(lookup)?;
    let variables = projects_group_set::Variables { projects, group };
    let moved = hub.run::<ProjectsGroupSet>(variables).await?;
    Ok(Value::Object(object([(
        "moved",
        json(&moved.set_projects_group),
    )])))
}
