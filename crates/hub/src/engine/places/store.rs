//! All SQL on schema `places`: I/O only.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::model::{Category, Centre, Draft, Place};
use crate::ids::{PlaceId, SiteId};

const COLUMNS: &str = "id, parent_id, name, latitude, longitude, radius_m, category, icon, \
                       aliases, geocoder_name, merged_into, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct PlaceRow {
    id: PlaceId,
    parent_id: Option<PlaceId>,
    name: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_m: Option<i32>,
    category: String,
    icon: String,
    aliases: Vec<String>,
    geocoder_name: Option<String>,
    merged_into: Option<PlaceId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PlaceRow> for Place {
    type Error = sqlx::Error;

    fn try_from(row: PlaceRow) -> Result<Self, Self::Error> {
        let unreadable = |column: &str| sqlx::Error::Decode(format!("bad place {column}").into());
        let centre = match (row.latitude, row.longitude, row.radius_m) {
            (Some(latitude), Some(longitude), Some(radius)) => Some(Centre {
                latitude,
                longitude,
                radius_m: u32::try_from(radius).map_err(|_| unreadable("radius"))?,
            }),
            (None, None, _) => None,
            _ => return Err(unreadable("centre")),
        };
        Ok(Self {
            id: row.id,
            parent: row.parent_id,
            name: row.name,
            centre,
            category: Category::parse(&row.category).ok_or_else(|| unreadable("category"))?,
            icon: row.icon,
            aliases: row.aliases,
            geocoder_name: row.geocoder_name,
            merged_into: row.merged_into,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn numbers(draft: &Draft) -> (Option<f64>, Option<f64>, Option<i32>) {
    let centre = draft.centre;
    (
        centre.map(|c| c.latitude),
        centre.map(|c| c.longitude),
        centre.and_then(|c| i32::try_from(c.radius_m).ok()),
    )
}

pub(super) async fn insert(
    conn: &mut PgConnection,
    id: PlaceId,
    draft: &Draft,
    at: DateTime<Utc>,
) -> Result<Place, sqlx::Error> {
    let (latitude, longitude, radius) = numbers(draft);
    let row: PlaceRow = sqlx::query_as(&format!(
        "INSERT INTO places.place (id, parent_id, name, latitude, longitude, radius_m, \
         category, icon, aliases, geocoder_name, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11) RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(draft.parent)
    .bind(&draft.name)
    .bind(latitude)
    .bind(longitude)
    .bind(radius)
    .bind(draft.category.as_str())
    .bind(&draft.icon)
    .bind(&draft.aliases)
    .bind(&draft.geocoder_name)
    .bind(at)
    .fetch_one(conn)
    .await?;
    row.try_into()
}

pub(super) async fn update(
    conn: &mut PgConnection,
    id: PlaceId,
    draft: &Draft,
    at: DateTime<Utc>,
) -> Result<Place, sqlx::Error> {
    let (latitude, longitude, radius) = numbers(draft);
    let row: PlaceRow = sqlx::query_as(&format!(
        "UPDATE places.place SET parent_id = $2, name = $3, latitude = $4, longitude = $5, \
         radius_m = $6, category = $7, icon = $8, aliases = $9, geocoder_name = $10, \
         updated_at = $11 WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(draft.parent)
    .bind(&draft.name)
    .bind(latitude)
    .bind(longitude)
    .bind(radius)
    .bind(draft.category.as_str())
    .bind(&draft.icon)
    .bind(&draft.aliases)
    .bind(&draft.geocoder_name)
    .bind(at)
    .fetch_one(conn)
    .await?;
    row.try_into()
}

/// The place; with `lock` the row stays locked until the transaction ends.
pub(super) async fn get(
    conn: &mut PgConnection,
    id: PlaceId,
    lock: bool,
) -> Result<Option<Place>, sqlx::Error> {
    let suffix = if lock { " FOR UPDATE" } else { "" };
    let row: Option<PlaceRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM places.place WHERE id = $1{suffix}"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(Place::try_from).transpose()
}

/// The places that are not merged, by name.
pub(super) async fn list(db: impl PgExecutor<'_>) -> Result<Vec<Place>, sqlx::Error> {
    let rows: Vec<PlaceRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM places.place WHERE merged_into IS NULL ORDER BY name, id"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Place::try_from).collect()
}

/// `id` and then its ancestors, nearest first.
pub(super) async fn chain(
    conn: &mut PgConnection,
    id: PlaceId,
) -> Result<Vec<PlaceId>, sqlx::Error> {
    sqlx::query_scalar(
        "WITH RECURSIVE up AS (SELECT id, parent_id, 1 AS n FROM places.place WHERE id = $1 \
         UNION ALL SELECT p.id, p.parent_id, up.n + 1 FROM places.place p \
         JOIN up ON p.id = up.parent_id) SELECT id FROM up ORDER BY n",
    )
    .bind(id)
    .fetch_all(conn)
    .await
}

/// How many levels of places lie below `id`; 0 for a place with no child.
pub(super) async fn height(conn: &mut PgConnection, id: PlaceId) -> Result<i32, sqlx::Error> {
    let levels: Option<i32> = sqlx::query_scalar(
        "WITH RECURSIVE down AS (SELECT id, 0 AS n FROM places.place WHERE id = $1 \
         UNION ALL SELECT p.id, down.n + 1 FROM places.place p JOIN down ON p.parent_id = down.id) \
         SELECT max(n) FROM down",
    )
    .bind(id)
    .fetch_one(conn)
    .await?;
    Ok(levels.unwrap_or(0))
}

/// How many child places and how many site rows point at `id`.
pub(super) async fn count_references(
    conn: &mut PgConnection,
    id: PlaceId,
) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM places.place WHERE parent_id = $1), \
         (SELECT count(*) FROM places.site_place WHERE place_id = $1)",
    )
    .bind(id)
    .fetch_one(conn)
    .await
}

/// Moves the children of `from` (all, or only those in `only`) to `to`; returns how many.
pub(super) async fn move_children(
    conn: &mut PgConnection,
    (from, to): (PlaceId, Option<PlaceId>),
    only: Option<&[Uuid]>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE places.place SET parent_id = $2 \
         WHERE parent_id = $1 AND ($3::uuid[] IS NULL OR id = ANY($3))",
    )
    .bind(from)
    .bind(to)
    .bind(only)
    .execute(conn)
    .await
    .map(|done| done.rows_affected())
}

/// Moves the site rows of `from` (all, or only those in `only`) to `to`; returns how many.
pub(super) async fn move_sites(
    conn: &mut PgConnection,
    (from, to): (PlaceId, PlaceId),
    only: Option<&[Uuid]>,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE places.site_place SET place_id = $2 \
         WHERE place_id = $1 AND ($3::uuid[] IS NULL OR site_id = ANY($3))",
    )
    .bind(from)
    .bind(to)
    .bind(only)
    .execute(conn)
    .await
    .map(|done| done.rows_affected())
}

/// Marks `from` as merged into `into`, and points every place already merged into `from` at
/// `into`, so a merge chain is never longer than one step.
pub(super) async fn set_merged(
    conn: &mut PgConnection,
    (from, into): (PlaceId, PlaceId),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE places.place SET merged_into = $2 WHERE merged_into = $1")
        .bind(from)
        .bind(into)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "UPDATE places.place SET merged_into = $2, parent_id = NULL, updated_at = $3 \
         WHERE id = $1",
    )
    .bind(from)
    .bind(into)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

pub(super) async fn set_aliases(
    conn: &mut PgConnection,
    id: PlaceId,
    aliases: &[String],
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE places.place SET aliases = $2, updated_at = $3 WHERE id = $1")
        .bind(id)
        .bind(aliases)
        .bind(at)
        .execute(conn)
        .await
        .map(drop)
}

/// Deletes the place and the places that were merged into it.
pub(super) async fn delete(conn: &mut PgConnection, id: PlaceId) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM places.place WHERE merged_into = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM places.place WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map(drop)
}

pub(super) async fn site_place(
    conn: &mut PgConnection,
    site: SiteId,
) -> Result<Option<PlaceId>, sqlx::Error> {
    sqlx::query_scalar("SELECT place_id FROM places.site_place WHERE site_id = $1 FOR UPDATE")
        .bind(site)
        .fetch_optional(conn)
        .await
}

pub(super) async fn put_site(
    conn: &mut PgConnection,
    site: SiteId,
    place: PlaceId,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO places.site_place (site_id, place_id) VALUES ($1, $2) \
         ON CONFLICT (site_id) DO UPDATE SET place_id = EXCLUDED.place_id",
    )
    .bind(site)
    .bind(place)
    .execute(conn)
    .await
    .map(drop)
}

pub(super) async fn drop_site(conn: &mut PgConnection, site: SiteId) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM places.site_place WHERE site_id = $1")
        .bind(site)
        .execute(conn)
        .await
        .map(drop)
}

/// The sites that point at each place, for the places shown.
pub(super) async fn sites_of(
    db: impl PgExecutor<'_>,
    places: &[Uuid],
) -> Result<Vec<(PlaceId, SiteId)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT place_id, site_id FROM places.site_place WHERE place_id = ANY($1) \
         ORDER BY site_id",
    )
    .bind(places)
    .fetch_all(db)
    .await
}

/// The title of each of `ids` that is a place; a merged place gives the title of the place it
/// was merged into.
pub(super) async fn titles(
    db: impl PgExecutor<'_>,
    ids: &[Uuid],
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.id, coalesce(m.name, p.name) FROM places.place p \
         LEFT JOIN places.place m ON m.id = p.merged_into WHERE p.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(db)
    .await
}
