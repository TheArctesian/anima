//! A group by its name or id, and projects by their slugs: what the command and the MCP tools
//! both need before a change (PRO-059, PRO-060).

use super::documents::{ProjectGroups, ProjectIdBySlug, project_groups, project_id_by_slug};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;

/// Each group, in name order.
///
/// # Errors
/// The hub's refusal.
pub async fn groups(hub: &Hub) -> Result<Vec<project_groups::ProjectGroupsProjectGroups>, Failure> {
    Ok(hub
        .run::<ProjectGroups>(project_groups::Variables)
        .await?
        .project_groups)
}

/// The id of the group whose name is `given` (the case of the letters is ignored) or whose id
/// is `given`.
///
/// # Errors
/// [`Failure::NotFound`] when no group has that name or id; the hub's refusal.
pub async fn group_id(hub: &Hub, given: &str) -> Result<String, Failure> {
    let all = groups(hub).await?;
    all.iter()
        .find(|group| group.id == given || group.name.to_lowercase() == given.trim().to_lowercase())
        .map(|group| group.id.clone())
        .ok_or_else(|| Failure::NotFound {
            message: format!("There is no project group named {}.", given.trim()),
            hint: Some(Hint::run("anima project group list")),
        })
}

/// The id of each project in `slugs`, in their order.
///
/// # Errors
/// [`Failure::NotFound`] for a slug that names no project; the hub's refusal.
pub async fn project_ids(hub: &Hub, slugs: &[String]) -> Result<Vec<String>, Failure> {
    let mut ids = Vec::new();
    for slug in slugs {
        let project = hub
            .run::<ProjectIdBySlug>(project_id_by_slug::Variables { slug: slug.clone() })
            .await?
            .project;
        let project = project.ok_or_else(|| Failure::NotFound {
            message: format!("There is no project with the slug {slug}."),
            hint: Some(Hint::run("anima project list")),
        })?;
        ids.push(project.id);
    }
    Ok(ids)
}
