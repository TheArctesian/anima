//! Merge, split and delete: the operations that move the references of a place (PLC-004,
//! PLC-005).

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{
    Divided, Entry, Error, Fault, Holders, Merged, Moved, References, Removal, Split, invalid,
    live, model::Place, shape, store,
};
use crate::{
    engine::{
        links,
        records::{Reference, Subject},
    },
    ids::PlaceId,
};

fn reference(id: PlaceId) -> Reference {
    Subject::Place(id).reference()
}

fn count(total: i64) -> u64 {
    u64::try_from(total).unwrap_or_default()
}

/// How many references `id` has, by sort (PLC-005).
///
/// # Errors
/// `Database`, `Links`, `Holder`.
pub async fn references(
    conn: &mut PgConnection,
    holders: &Holders,
    id: PlaceId,
) -> Result<References, Error> {
    let mut held = Vec::new();
    for holder in holders.all() {
        held.push((holder.name(), holder.count(conn, id).await?));
    }
    let (children, sites) = store::count_references(conn, id).await?;
    Ok(References {
        links: links::count(conn, &reference(id)).await?,
        sites: count(sites),
        children: count(children),
        held,
    })
}

/// Moves every reference of `from` to `into` in the caller's transaction, adds the name and the
/// aliases of `from` to `into`, and marks `from` as merged. A link that would join a record to
/// itself, or repeat one that exists, is removed (PLC-004).
///
/// # Errors
/// `NotFound`, `Merged`; `Invalid` when `from` is `into`, when `into` is inside `from`, or when
/// the tree would be deeper than 5 levels; `Database`, `Links`, `Holder`. The caller rolls back.
pub async fn merge(
    conn: &mut PgConnection,
    holders: &Holders,
    (from, into): (PlaceId, PlaceId),
    at: DateTime<Utc>,
) -> Result<Merged, Error> {
    if from == into {
        return Err(invalid("into", "Give a different place."));
    }
    store::lock_tree(conn).await?;
    // Lock in the order of the IDs, so two merges of the same pair cannot wait on each other.
    let (first, second) = if Uuid::from(from) < Uuid::from(into) {
        (from, into)
    } else {
        (into, from)
    };
    let (a, b) = (live(conn, first).await?, live(conn, second).await?);
    let (merged, remaining) = if first == from { (a, b) } else { (b, a) };
    let chain = store::chain(conn, into).await?;
    if chain.contains(&from) {
        return Err(invalid("into", "Give a place that is not inside this one."));
    }
    let height = usize::try_from(store::height(conn, from).await?).unwrap_or_default();
    if !shape::fits(chain.len(), height) {
        return Err(invalid("into", "Places go at most 5 levels deep."));
    }
    let moved = Moved {
        links: links::retarget(conn, &reference(from), &reference(into)).await?,
        sites: store::move_sites(conn, (from, into), None).await?,
        children: store::move_children(conn, (from, Some(into)), None).await?,
        held: move_held(conn, holders, (from, into)).await?,
    };
    let aliases = shape::merged_aliases(&remaining, &merged);
    store::set_aliases(conn, into, &aliases, at).await?;
    store::set_merged(conn, (from, into), at).await?;
    let place = Place {
        aliases,
        updated_at: at,
        ..remaining
    };
    let from_place = Place {
        merged_into: Some(into),
        parent: None,
        updated_at: at,
        ..merged
    };
    Ok(Merged {
        from: from_place,
        into: place,
        moved,
    })
}

async fn move_held(
    conn: &mut PgConnection,
    holders: &Holders,
    (from, into): (PlaceId, PlaceId),
) -> Result<Vec<(&'static str, u64)>, Error> {
    let mut held = Vec::new();
    for holder in holders.all() {
        held.push((holder.name(), holder.move_all(conn, from, into).await?));
    }
    Ok(held)
}

/// Makes one new top-level place from `split.new` and moves the named children, site rows and
/// record links from `id` to it. A reference that is not named stays (PLC-004).
///
/// # Errors
/// `NotFound`, `Merged`; `Invalid` for a field of the new place, or for a named child, site or
/// link that is not a reference of `id`; `Database`, `Links`. The caller rolls back.
pub async fn split(
    conn: &mut PgConnection,
    id: PlaceId,
    split: Split,
    at: DateTime<Utc>,
) -> Result<Divided, Error> {
    store::lock_tree(conn).await?;
    let origin = live(conn, id).await?;
    let draft = shape::check(Entry {
        parent: None,
        ..split.new
    })
    .map_err(Error::Invalid)?;
    let new = store::insert(conn, Uuid::now_v7().into(), &draft, at).await?;
    let children: Vec<Uuid> = unique(split.children.iter().map(|c| Uuid::from(*c)));
    let sites: Vec<Uuid> = unique(split.sites.iter().map(|s| Uuid::from(*s)));
    let named_links = unique(split.links.iter().copied());
    let moved = Moved {
        children: store::move_children(conn, (id, Some(new.id)), Some(&children)).await?,
        sites: store::move_sites(conn, (id, new.id), Some(&sites)).await?,
        links: links::retarget_named(conn, &named_links, (&reference(id), &reference(new.id)))
            .await?,
        held: Vec::new(),
    };
    if moved.children != children.len() as u64 {
        return Err(invalid("children", "Name only children of this place."));
    }
    if moved.sites != sites.len() as u64 {
        return Err(invalid("sites", "Name only sites of this place."));
    }
    if moved.links != named_links.len() as u64 {
        return Err(invalid("links", "Name only links of this place."));
    }
    Ok(Divided {
        from: origin,
        new,
        moved,
    })
}

fn unique<T: PartialEq>(ids: impl Iterator<Item = T>) -> Vec<T> {
    let mut kept: Vec<T> = Vec::new();
    for id in ids {
        if !kept.contains(&id) {
            kept.push(id);
        }
    }
    kept
}

/// Deletes a place that has no reference. A place with references is merged into
/// `replacement`; with none given the answer is `InUse` with the counts (PLC-005).
///
/// # Errors
/// `NotFound`, `Merged`, `InUse`; `Invalid` when the replacement is the place itself or is not
/// usable (as [`merge`]); `Database`, `Links`, `Holder`. The caller rolls back.
pub async fn remove(
    conn: &mut PgConnection,
    holders: &Holders,
    (id, replacement): (PlaceId, Option<PlaceId>),
    at: DateTime<Utc>,
) -> Result<Removal, Error> {
    store::lock_tree(conn).await?;
    let place = live(conn, id).await?;
    let found = references(conn, holders, id).await?;
    match (found.total(), replacement) {
        (0, _) => {
            store::delete(conn, id).await?;
            Ok(Removal::Deleted(place))
        }
        (_, None) => Err(Error::InUse(found)),
        (_, Some(into)) => {
            let merged = merge(conn, holders, (id, into), at).await;
            Ok(Removal::Replaced(Box::new(merged.map_err(of_replacement)?)))
        }
    }
}

/// A refusal of the merge of a delete, named for the argument `replacement`: the place itself
/// was found, so a place that is missing or merged is the replacement.
fn of_replacement(error: Error) -> Error {
    match error {
        Error::Invalid(faults) => Error::Invalid(
            faults
                .into_iter()
                .map(|fault| Fault {
                    field: "replacement",
                    ..fault
                })
                .collect(),
        ),
        Error::NotFound | Error::Merged => {
            invalid("replacement", "Give a place that exists and is not merged.")
        }
        other => other,
    }
}
