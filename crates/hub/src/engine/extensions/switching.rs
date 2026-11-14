//! The rule for a requested switch: which switches change, or why none does (EXT-011,
//! EXT-012). Pure: it reads the registry and the set of keys that are on, and writes nothing.

use std::collections::HashSet;

use super::model::{Entry, Key};

/// Why a requested switch is refused. Every one is `CONFLICT` at the API.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The extension, or one it needs, is listed but not built.
    #[error("{} is not built yet.", .0.name())]
    NotBuilt(Key),
    /// Part of the engine: always on, no switch.
    #[error("{} is part of the engine and always on.", .0.name())]
    Fixed(Key),
    /// Extensions that are on need this one (EXT-012).
    #[error("{}", needed_message(.key, .by))]
    Needed { key: Key, by: Vec<Key> },
}

fn needed_message(key: &Key, by: &[Key]) -> String {
    let names = by
        .iter()
        .map(|key| key.name())
        .collect::<Vec<_>>()
        .join(" and ");
    let (is, needs) = if by.len() == 1 {
        ("is", "needs")
    } else {
        ("are", "need")
    };
    format!(
        "{names} {is} on and {needs} {}. Set {names} off first.",
        key.name()
    )
}

fn find(registry: &[Entry], key: Key) -> &Entry {
    registry
        .iter()
        .find(|entry| entry.key == key)
        .expect("a Key is made from the registry")
}

/// The keys whose switch changes when `key` is set to `want`, the named one first. Empty when
/// nothing changes. `on_now` holds the keys that are on, the fixed ones included.
///
/// # Errors
/// A [`Refusal`] when `key` or a need is not built, when `key` is fixed, or when turning `key`
/// off would leave an extension that is on without a need.
pub fn plan(
    registry: &[Entry],
    on_now: &HashSet<Key>,
    key: Key,
    want: bool,
) -> Result<Vec<Key>, Refusal> {
    let entry = find(registry, key);
    if entry.fixed {
        return Err(Refusal::Fixed(key));
    }
    if want {
        turn_on(registry, on_now, entry)
    } else {
        turn_off(registry, on_now, entry)
    }
}

fn turn_on(registry: &[Entry], on_now: &HashSet<Key>, entry: &Entry) -> Result<Vec<Key>, Refusal> {
    if let Some(unbuilt) = (std::iter::once(entry.key).chain(entry.needs.iter().copied()))
        .find(|key| !find(registry, *key).built)
    {
        return Err(Refusal::NotBuilt(unbuilt));
    }
    Ok(std::iter::once(entry.key)
        .chain(entry.needs.iter().copied())
        .filter(|key| !on_now.contains(key))
        .collect())
}

fn turn_off(registry: &[Entry], on_now: &HashSet<Key>, entry: &Entry) -> Result<Vec<Key>, Refusal> {
    if !entry.built {
        return Err(Refusal::NotBuilt(entry.key));
    }
    let by: Vec<Key> = needed_by(registry, on_now, entry.key);
    if !by.is_empty() {
        return Err(Refusal::Needed { key: entry.key, by });
    }
    Ok(if on_now.contains(&entry.key) {
        vec![entry.key]
    } else {
        Vec::new()
    })
}

/// The keys that are on and list `key` among their needs, in registry order.
#[must_use]
pub fn needed_by(registry: &[Entry], on_now: &HashSet<Key>, key: Key) -> Vec<Key> {
    registry
        .iter()
        .filter(|other| on_now.contains(&other.key) && other.needs.contains(&key))
        .map(|other| other.key)
        .collect()
}
