//! The repository an operator names: by `owner/name` (PRJ-016) or by ID.

use super::documents::{ProjectList, project_list};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::paging::{MAX_PAGE, fetch};

/// The ID of the repository `name` is the full name or the ID of. A name with a `/` is looked
/// up in the projects, those with repositories left out too; anything else is taken as an ID.
///
/// # Errors
/// [`Failure::NotFound`] when no project holds a repository with that full name; the hub's
/// refusal.
pub async fn resolve(hub: &Hub, name: &str) -> Result<String, Failure> {
    if !name.contains('/') {
        return Ok(name.to_owned());
    }
    for left_out in [false, true] {
        let projects = fetch(None, true, move |after| async move {
            let variables = project_list::Variables {
                left_out: Some(left_out),
                first: Some(MAX_PAGE),
                after,
            };
            Ok(hub.run::<ProjectList>(variables).await?.projects)
        })
        .await?;
        let found = projects
            .edges
            .iter()
            .flat_map(|edge| &edge.node.repositories)
            .find(|repository| repository.full_name == name);
        if let Some(repository) = found {
            return Ok(repository.id.clone());
        }
    }
    Err(Failure::NotFound {
        message: format!("No project has a repository named {name}."),
        hint: Some(Hint::run("anima project list --all")),
    })
}
