//! Places: where things are, as every extension names them (K-E4 Section 8; PLC-001 to
//! PLC-006). A place is a record with the reference `place:<PlaceId>`; the operator's name is
//! its title and a geocoder never changes it.

pub mod graphql;
mod inputs;
mod merge;
mod model;
mod shape;
mod store;
mod trail;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub use merge::{merge, references, remove, split};
pub use model::{
    Category, Centre, Divided, Draft, Entry, Fault, Merged, Moved, Patch, Place, References,
    Removal, Siting, Split,
};

use crate::{
    engine::links::{self, Titles},
    ids::{PlaceId, SiteId},
};

/// A holder of a `PlaceId` could not answer now. The message holds no detail.
#[derive(Debug, thiserror::Error)]
#[error("the {0} domain could not count or move its places")]
pub struct Unavailable(pub &'static str);

/// Why a places operation failed. Messages hold no SQL and no input value.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// Some input is not valid; each [`Fault`] names a field.
    #[error("the place input is not valid")]
    Invalid(Vec<Fault>),
    #[error("no such place")]
    NotFound,
    /// The place was merged into another one and cannot be changed (PLC-004).
    #[error("the place was merged into another place")]
    Merged,
    /// The place has references and no replacement was given (PLC-005).
    #[error("the place has references")]
    InUse(References),
    #[error(transparent)]
    Links(#[from] links::Error),
    #[error(transparent)]
    Holder(#[from] Unavailable),
}

/// A later domain that keeps a `PlaceId` in its own table. It gives itself to the places
/// domain at wiring, so that `places` never imports its store (PLC-004, PLC-005).
#[async_trait]
pub trait Holder: Send + Sync {
    /// The domain, for the answer of a refused delete.
    fn name(&self) -> &'static str;

    /// How many rows of the domain hold `place`.
    ///
    /// # Errors
    /// [`Unavailable`] when the domain cannot count.
    async fn count(&self, conn: &mut PgConnection, place: PlaceId) -> Result<u64, Unavailable>;

    /// Points every row that holds `from` at `to`; returns how many.
    ///
    /// # Errors
    /// [`Unavailable`] when the domain cannot move its rows.
    async fn move_all(
        &self,
        conn: &mut PgConnection,
        from: PlaceId,
        to: PlaceId,
    ) -> Result<u64, Unavailable>;
}

/// Every [`Holder`] of the hub, built once in wiring.
#[derive(Clone, Default)]
pub struct Holders(Vec<Arc<dyn Holder>>);

impl Holders {
    #[must_use]
    pub fn new(holders: Vec<Arc<dyn Holder>>) -> Self {
        Self(holders)
    }

    fn all(&self) -> &[Arc<dyn Holder>] {
        &self.0
    }
}

fn invalid(field: &'static str, message: &'static str) -> Error {
    Error::Invalid(vec![Fault { field, message }])
}

/// The place `id`, which must exist and not be merged; its row is locked.
async fn live(conn: &mut PgConnection, id: PlaceId) -> Result<Place, Error> {
    let place = store::get(conn, id, true).await?.ok_or(Error::NotFound)?;
    if place.merged_into.is_some() {
        Err(Error::Merged)
    } else {
        Ok(place)
    }
}

/// How deep `parent` sits (a top-level place is 1), when a place may go inside it.
async fn parent_depth(
    conn: &mut PgConnection,
    parent: PlaceId,
    moving: Option<PlaceId>,
) -> Result<usize, Error> {
    let inside = match live(conn, parent).await {
        Ok(place) => place,
        Err(Error::NotFound | Error::Merged) => {
            return Err(invalid("parentId", "Give a place that exists."));
        }
        Err(other) => return Err(other),
    };
    let chain = store::chain(conn, inside.id).await?;
    if moving.is_some_and(|id| chain.contains(&id)) {
        return Err(invalid("parentId", "A place cannot be inside itself."));
    }
    Ok(chain.len())
}

/// Makes a place (PLC-001, PLC-002).
///
/// # Errors
/// `Invalid` for a field to fix, a parent that does not exist or a tree deeper than 5 levels;
/// `Database`. The caller rolls back.
pub async fn create(
    conn: &mut PgConnection,
    entry: Entry,
    at: DateTime<Utc>,
) -> Result<Place, Error> {
    let draft = shape::check(entry).map_err(Error::Invalid)?;
    if let Some(parent) = draft.parent {
        store::lock_tree(conn).await?;
        let depth = parent_depth(conn, parent, None).await?;
        if !shape::fits(depth + 1, 0) {
            return Err(invalid("parentId", "Places go at most 5 levels deep."));
        }
    }
    Ok(store::insert(conn, Uuid::now_v7().into(), &draft, at).await?)
}

/// Changes a place (PLC-001 to PLC-003). Returns the place and the names of the fields that
/// changed; nothing is written when none did.
///
/// # Errors
/// `NotFound`, `Merged`, `Invalid` as [`create`], and a place inside itself; `Database`.
pub async fn update(
    conn: &mut PgConnection,
    id: PlaceId,
    patch: Patch,
    at: DateTime<Utc>,
) -> Result<(Place, Vec<&'static str>), Error> {
    store::lock_tree(conn).await?;
    let before = live(conn, id).await?;
    let draft = shape::apply(&before, patch).map_err(Error::Invalid)?;
    let changed = shape::changes(&before, &draft);
    if changed.is_empty() {
        return Ok((before, changed));
    }
    if let (Some(parent), true) = (draft.parent, draft.parent != before.parent) {
        let depth = parent_depth(conn, parent, Some(id)).await?;
        let height = usize::try_from(store::height(conn, id).await?).unwrap_or_default();
        if !shape::fits(depth + 1, height) {
            return Err(invalid("parentId", "Places go at most 5 levels deep."));
        }
    }
    Ok((store::update(conn, id, &draft, at).await?, changed))
}

/// Links a site to a place, or with `None` removes the link (PLC-006). One site has at most one
/// place.
///
/// # Errors
/// `NotFound` when the place does not exist; `Merged`; `Database`.
pub async fn set_site(
    conn: &mut PgConnection,
    site: SiteId,
    place: Option<PlaceId>,
) -> Result<Siting, Error> {
    if let Some(place) = place {
        live(conn, place).await?;
    }
    let before = store::site_place(conn, site).await?;
    match place {
        Some(place) if before != Some(place) => store::put_site(conn, site, place).await?,
        None if before.is_some() => store::drop_site(conn, site).await?,
        _ => {}
    }
    Ok(Siting {
        site,
        before,
        after: place,
    })
}

/// The places that are not merged, by name.
///
/// # Errors
/// `Database`.
pub async fn list(db: &PgPool) -> Result<Vec<Place>, Error> {
    Ok(store::list(db).await?)
}

/// The place `id`, or the one it was merged into: `place:<id>` still resolves after a merge
/// (PLC-004). `None` when no place has this ID.
///
/// # Errors
/// `Database`.
pub async fn current(db: &PgPool, id: PlaceId) -> Result<Option<Place>, Error> {
    let mut conn = db.acquire().await?;
    let mut place = store::get(&mut conn, id, false).await?;
    // A merge never makes a chain, but a loop would be the data's fault, so the walk is bounded.
    for _ in 0..8 {
        let Some(next) = place.as_ref().and_then(|p| p.merged_into) else {
            break;
        };
        place = store::get(&mut conn, next, false).await?;
    }
    Ok(place)
}

/// The place of `site`, when the site is linked to one (PLC-006). The row is locked until
/// the caller's transaction ends.
///
/// # Errors
/// `Database`.
pub async fn place_of_site(
    conn: &mut PgConnection,
    site: SiteId,
) -> Result<Option<PlaceId>, Error> {
    Ok(store::site_place(conn, site).await?)
}

/// The sites that point at each of `places` (PLC-006).
///
/// # Errors
/// `Database`.
pub async fn sites_of(
    db: &PgPool,
    places: &[PlaceId],
) -> Result<HashMap<PlaceId, Vec<SiteId>>, Error> {
    let ids: Vec<Uuid> = places.iter().map(|id| Uuid::from(*id)).collect();
    let mut found: HashMap<PlaceId, Vec<SiteId>> = HashMap::new();
    for (place, site) in store::sites_of(db, &ids).await? {
        found.entry(place).or_default().push(site);
    }
    Ok(found)
}

/// The titles of the records of kind `place`, for `links` (K-E4 Section 4).
struct PlaceTitles;

#[async_trait]
impl Titles for PlaceTitles {
    fn kind(&self) -> &'static str {
        "place"
    }

    async fn titles(
        &self,
        db: &PgPool,
        ids: &[String],
    ) -> Result<HashMap<String, String>, links::Unavailable> {
        let parsed: Vec<Uuid> = ids.iter().filter_map(|id| id.parse().ok()).collect();
        let rows = store::titles(db, &parsed)
            .await
            .map_err(|_| links::Unavailable("place"))?;
        Ok(rows
            .into_iter()
            .map(|(id, name)| (id.hyphenated().to_string(), name))
            .collect())
    }
}

/// The places' [`Titles`], for the wiring of `links`.
#[must_use]
pub fn record_titles() -> Arc<dyn Titles> {
    Arc::new(PlaceTitles)
}
