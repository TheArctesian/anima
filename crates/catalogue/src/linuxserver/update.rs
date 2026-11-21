//! The one change that the generator may make to a recipe that exists (SF-244): a
//! new digest or a new version. Each other change needs a person who reads it.

use serde_json::Value;

use crate::{
    Recipe,
    entry::{Entry, MachineTest, Reviewed},
};

/// Why the generator writes nothing to a recipe that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Change {
    #[error("the recipe was written by a person")]
    Reviewed,
    #[error("the recipe cannot be read")]
    NotReadable,
    #[error("the change is more than a new digest or a new version, so a person reads it")]
    NotOnlyPin,
}

fn json(text: Option<&str>) -> Result<Value, Change> {
    text.and_then(|text| serde_json::from_str(text).ok())
        .ok_or(Change::NotReadable)
}

/// `value` at `path`, a key at each step.
fn at<'a>(value: &'a mut Value, path: &[&str]) -> Option<&'a mut Value> {
    path.iter().try_fold(value, |node, key| node.get_mut(*key))
}

/// Copies the value at `path` of `from` into `into`.
fn copy(into: &mut Value, from: &Value, path: &[&str]) -> Result<(), Change> {
    let new = path
        .iter()
        .try_fold(from, |node, key| node.get(*key))
        .ok_or(Change::NotOnlyPin)?
        .clone();
    *at(into, path).ok_or(Change::NotOnlyPin)? = new;
    Ok(())
}

/// The images of an entry: each service and its reference.
fn images(entry: &Value) -> Vec<&str> {
    entry
        .pointer("/anima/images")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |images| {
            images
                .iter()
                .filter_map(|image| image.get("image").and_then(Value::as_str))
                .collect()
        })
}

fn entry(existing: &Recipe) -> Option<Entry> {
    (existing.entry.as_deref()).and_then(|text| serde_json::from_str::<Entry>(text).ok())
}

/// The review state of a recipe that exists, which the generator writes again unchanged
/// (SF-255).
#[must_use]
pub fn reviewed(existing: &Recipe) -> Option<Reviewed> {
    entry(existing).and_then(|entry| entry.anima.reviewed)
}

/// The last test on a machine of a recipe that exists, which the generator keeps only for the
/// same digest.
#[must_use]
pub fn tested(existing: &Recipe) -> Option<MachineTest> {
    entry(existing).map(|entry| entry.anima.last_test)
}

/// Whether the generator may write `made` over `existing`.
///
/// # Errors
/// [`Change::Reviewed`] for a recipe with no `origin`, which a person wrote;
/// [`Change::NotReadable`] for an entry that is not JSON; [`Change::NotOnlyPin`] when the two
/// recipes differ in more than the version, the images, the days of the reads and the pin, and
/// a reset of the last test on a machine to `not-tested`.
pub fn update(existing: &Recipe, made: &Recipe) -> Result<(), Change> {
    let mut old = json(existing.entry.as_deref())?;
    let new = json(made.entry.as_deref())?;
    if old.pointer("/anima/origin").is_none_or(Value::is_null) {
        return Err(Change::Reviewed);
    }
    let mut compose = existing.compose.clone().unwrap_or_default();
    let (old_images, new_images) = (images(&old), images(&new));
    if old_images.len() != new_images.len() {
        return Err(Change::NotOnlyPin);
    }
    for (before, after) in old_images.iter().zip(&new_images) {
        compose = compose.replace(before, after);
    }
    for path in [
        &["version"][..],
        &["anima", "pinned"],
        &["anima", "origin", "read"],
        &["anima", "images"],
    ] {
        copy(&mut old, &new, path)?;
    }
    if new.pointer("/anima/last_test") == Some(&serde_json::json!({ "result": "not-tested" })) {
        copy(&mut old, &new, &["anima", "last_test"])?;
    }
    if old == new && Some(compose) == made.compose && existing.environment == made.environment {
        Ok(())
    } else {
        Err(Change::NotOnlyPin)
    }
}
