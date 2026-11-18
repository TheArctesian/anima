//! The SQL on `places.site_place`: which site is at which place (PLC-006).

use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::ids::{PlaceId, SiteId};

/// Moves the site rows of `from` (all, or only those in `only`) to `to`; returns how many.
pub(in crate::engine::places) async fn move_sites(
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

pub(in crate::engine::places) async fn site_place(
    conn: &mut PgConnection,
    site: SiteId,
) -> Result<Option<PlaceId>, sqlx::Error> {
    sqlx::query_scalar("SELECT place_id FROM places.site_place WHERE site_id = $1 FOR UPDATE")
        .bind(site)
        .fetch_optional(conn)
        .await
}

pub(in crate::engine::places) async fn put_site(
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

pub(in crate::engine::places) async fn drop_site(
    conn: &mut PgConnection,
    site: SiteId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM places.site_place WHERE site_id = $1")
        .bind(site)
        .execute(conn)
        .await
        .map(drop)
}

/// The sites that point at each place, for the places shown.
pub(in crate::engine::places) async fn sites_of(
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
