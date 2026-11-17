//! Two sources, one event: both entries stay and the lower-ranked one points to the
//! higher-ranked one (TML-005). Pure.

use std::collections::HashMap;

use super::model::{Category, Evidence};
use crate::engine::records::Reference;

/// The order that decides which entry of one event the others point to: the higher rank,
/// then the source name and the reference that sort first, so the answer never depends on
/// the order of `entries`.
fn outranks(a: &Evidence, b: &Evidence) -> bool {
    (b.rank, &a.source, a.reference.as_str()) < (a.rank, &b.source, b.reference.as_str())
}

/// Sets `superseded_by` on each entry that a different source outranks for the same event.
/// Two entries record the same event when they have the same category and the same
/// `occurrence`. Entries of the leading entry's own source and entries an adapter already
/// pointed elsewhere are left as they are. No entry is removed.
pub(super) fn supersede(entries: &mut [Evidence]) {
    let mut leaders: HashMap<(Category, &str), &Evidence> = HashMap::new();
    for entry in entries.iter() {
        let Some(occurrence) = entry.occurrence.as_deref() else {
            continue;
        };
        let leader = leaders.entry((entry.category, occurrence)).or_insert(entry);
        if outranks(entry, leader) {
            *leader = entry;
        }
    }
    let leaders: HashMap<(Category, String), (String, Reference)> = leaders
        .into_iter()
        .map(|((category, occurrence), leader)| {
            let leader = (leader.source.clone(), leader.reference.clone());
            ((category, occurrence.to_owned()), leader)
        })
        .collect();
    for entry in entries.iter_mut() {
        let Some(occurrence) = entry.occurrence.clone() else {
            continue;
        };
        let Some((source, reference)) = leaders.get(&(entry.category, occurrence)) else {
            continue;
        };
        if entry.superseded_by.is_none() && entry.source != *source {
            entry.superseded_by = Some(reference.clone());
        }
    }
}
