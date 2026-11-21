//! The entries that the hub lists: each recipe that passes the static test of task H36.

use catalogue::{Recipe, check, entry::Entry};

use super::model::Listed;

/// The name of each variable of an example environment file (`NAME=value`), never a value.
fn variable_names(example: &str) -> Vec<String> {
    example
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(name, _)| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// The recipe as a listed entry, when the static test finds nothing.
fn listed_one(recipe: Recipe) -> Option<Listed> {
    if !check(&recipe).is_empty() {
        return None;
    }
    let entry: Entry = serde_json::from_str(recipe.entry.as_deref()?).ok()?;
    Some(Listed {
        entry,
        compose: recipe.compose?,
        variables: variable_names(&recipe.environment?),
    })
}

/// Each recipe of `recipes` that passes the static test, in the order of the identifiers. A
/// recipe with a finding is not listed (HOST-047).
pub(super) fn listed(recipes: Vec<Recipe>) -> Vec<Listed> {
    let mut found: Vec<Listed> = recipes.into_iter().filter_map(listed_one).collect();
    found.sort_by(|left, right| left.entry.id.cmp(&right.entry.id));
    found
}
