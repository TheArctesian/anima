//! All SQL on schema `maintenance`: I/O only; rows are mapped in `records`. Ids and time come
//! in as parameters.

use chrono::{DateTime, Utc};
use sqlx::{Connection, PgConnection, PgExecutor, PgPool, types::Json};

use super::{
    Error, FieldError,
    model::{NewWindow, Target, Window, WindowRule},
    records::{WindowRow, to_windows, write_error},
};
use crate::ids::WindowId;

/// A window with its targets in one row, targets in byte order of type then value.
macro_rules! select_window {
    ($($tail:expr),*) => {
        concat!(
            "SELECT w.id, w.name, w.timezone, w.rule, w.active_manual, w.created_at, w.updated_at, \
                    ARRAY(SELECT t.target_type FROM maintenance.window_targets t \
                          WHERE t.window_id = w.id \
                          ORDER BY t.target_type, t.target COLLATE \"C\") AS target_types, \
                    ARRAY(SELECT t.target FROM maintenance.window_targets t \
                          WHERE t.window_id = w.id \
                          ORDER BY t.target_type, t.target COLLATE \"C\") AS targets \
             FROM maintenance.windows w ",
            $($tail),*
        )
    };
}

/// Every window, in byte order of the name.
pub(super) async fn all(db: &PgPool) -> Result<Vec<Window>, Error> {
    let rows = sqlx::query_as(select_window!("ORDER BY w.name COLLATE \"C\""))
        .fetch_all(db)
        .await?;
    to_windows(rows)
}

pub(super) async fn insert(
    conn: &mut PgConnection,
    id: WindowId,
    now: DateTime<Utc>,
    window: &NewWindow,
) -> Result<Window, Error> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $6)",
    )
    .bind(id)
    .bind(&window.name)
    .bind(window.rule.kind().as_str())
    .bind(window.timezone.name())
    .bind(Json(&window.rule))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(write_error)?;
    insert_targets(&mut tx, id, &window.targets).await?;
    let created = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(created)
}

async fn insert_targets(
    conn: &mut PgConnection,
    id: WindowId,
    targets: &[Target],
) -> Result<(), Error> {
    let types: Vec<&str> = targets.iter().map(|t| t.target_type().as_str()).collect();
    let values: Vec<String> = targets.iter().map(Target::value).collect();
    sqlx::query(
        "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
         SELECT $1, * FROM unnest($2::text[], $3::text[])",
    )
    .bind(id)
    .bind(types)
    .bind(values)
    .execute(conn)
    .await?;
    Ok(())
}

/// Replaces the name, time zone, rule and targets; returns the window as it was and as it is.
/// A window that stays manual keeps its on/off state; any other kind starts off.
pub(super) async fn replace(
    conn: &mut PgConnection,
    id: WindowId,
    now: DateTime<Utc>,
    window: &NewWindow,
) -> Result<(Window, Window), Error> {
    let mut tx = conn.begin().await?;
    let before = lock(&mut tx, id).await?;
    sqlx::query(
        "UPDATE maintenance.windows SET name = $2, kind = $3, timezone = $4, rule = $5, \
                active_manual = active_manual AND $3 = 'manual', updated_at = $6 WHERE id = $1",
    )
    .bind(id)
    .bind(&window.name)
    .bind(window.rule.kind().as_str())
    .bind(window.timezone.name())
    .bind(Json(&window.rule))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(write_error)?;
    sqlx::query("DELETE FROM maintenance.window_targets WHERE window_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    insert_targets(&mut tx, id, &window.targets).await?;
    let after = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok((before, after))
}

/// Deletes the window and, by foreign key, its targets. Returns the window as it was.
pub(super) async fn delete(conn: &mut PgConnection, id: WindowId) -> Result<Window, Error> {
    let mut tx = conn.begin().await?;
    let window = lock(&mut tx, id).await?;
    sqlx::query("DELETE FROM maintenance.windows WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(window)
}

/// Switches a manual window on or off.
///
/// # Errors
/// `Validation` when the window is not a manual one.
pub(super) async fn set_manual(
    conn: &mut PgConnection,
    id: WindowId,
    now: DateTime<Utc>,
    active: bool,
) -> Result<Window, Error> {
    let mut tx = conn.begin().await?;
    let window = lock(&mut tx, id).await?;
    if window.rule != WindowRule::Manual {
        return Err(Error::Validation(vec![FieldError::new(
            "id",
            "Only a manual window can be switched on or off.",
        )]));
    }
    sqlx::query("UPDATE maintenance.windows SET active_manual = $2, updated_at = $3 WHERE id = $1")
        .bind(id)
        .bind(active)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    let changed = fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(changed)
}

/// Reads the window and locks its row until the transaction ends.
async fn lock(conn: &mut PgConnection, id: WindowId) -> Result<Window, Error> {
    let row = sqlx::query_as::<_, WindowRow>(select_window!("WHERE w.id = $1 FOR UPDATE OF w"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(Error::NotFound(id))?;
    Ok(Window::try_from(row)?)
}

async fn fetch<'e>(db: impl PgExecutor<'e>, id: WindowId) -> Result<Window, Error> {
    let row = sqlx::query_as::<_, WindowRow>(select_window!("WHERE w.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound(id))?;
    Ok(Window::try_from(row)?)
}
