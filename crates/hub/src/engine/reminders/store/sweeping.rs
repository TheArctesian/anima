//! The SQL of the sweep: its lock, its cursors and the place states (K-E6 Sections 3 and 4).

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::engine::reminders::{model::PlaceChange, place_state::PlaceState};

/// Holds the sweep until the transaction ends, so that two sweeps do not read one cursor.
pub(in crate::engine::reminders) async fn lock_sweep(
    conn: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('reminders sweep', 2090))")
        .execute(conn)
        .await
        .map(drop)
}

pub(in crate::engine::reminders) async fn cursor(
    conn: &mut PgConnection,
    name: &str,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar("SELECT position FROM reminders.cursor WHERE name = $1")
        .bind(name)
        .fetch_optional(conn)
        .await
}

pub(in crate::engine::reminders) async fn set_cursor(
    conn: &mut PgConnection,
    name: &str,
    position: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reminders.cursor (name, position) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET position = excluded.position",
    )
    .bind(name)
    .bind(position)
    .execute(conn)
    .await
    .map(drop)
}

pub(in crate::engine::reminders) async fn place_states(
    conn: &mut PgConnection,
) -> Result<Vec<PlaceState>, sqlx::Error> {
    let rows: Vec<(Uuid, bool, DateTime<Utc>, Uuid)> =
        sqlx::query_as("SELECT place_id, inside, changed_at, sample_id FROM reminders.place_state")
            .fetch_all(conn)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(place, inside, changed_at, sample)| PlaceState {
            place: place.into(),
            inside,
            changed_at,
            sample: sample.into(),
        })
        .collect())
}

pub(in crate::engine::reminders) async fn put_place_state(
    conn: &mut PgConnection,
    change: &PlaceChange,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reminders.place_state (place_id, inside, changed_at, sample_id) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (place_id) DO UPDATE SET \
         inside = excluded.inside, changed_at = excluded.changed_at, \
         sample_id = excluded.sample_id",
    )
    .bind(Uuid::from(change.place))
    .bind(change.inside)
    .bind(change.observed_at)
    .bind(Uuid::from(change.sample))
    .execute(conn)
    .await
    .map(drop)
}

/// The last health that the sweep saw of each source account, as `(account, state)`.
pub(in crate::engine::reminders) async fn states(
    conn: &mut PgConnection,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT name, state FROM reminders.known WHERE sort = 'state' AND state IS NOT NULL",
    )
    .fetch_all(conn)
    .await
}

pub(in crate::engine::reminders) async fn remember_state(
    conn: &mut PgConnection,
    (account, state): (&str, &str),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reminders.known (sort, name, state, at) VALUES ('state', $1, $2, $3) \
         ON CONFLICT (sort, name) DO UPDATE SET state = excluded.state, at = excluded.at",
    )
    .bind(account)
    .bind(state)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}
