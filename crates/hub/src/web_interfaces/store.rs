//! All SQL on schema `web_interfaces`: I/O only. Ids and time come in as parameters. Writes
//! take the caller's connection so the caller can add its audit entry in the same transaction.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::{Connection, PgConnection, PgExecutor};

use super::{
    Error,
    model::{Missing, OpenIn, Pin, WebInterface},
    rules::{self, NewInterface},
};
use crate::ids::{CheckId, DeviceId, OperatorId, WebInterfaceId};

const SELECT: &str = "SELECT i.id, i.device_id, i.label, i.url, i.open_in, \
                             p.fingerprint_sha256, p.accepted_at \
                      FROM web_interfaces.interfaces i \
                      LEFT JOIN web_interfaces.pins p ON p.interface_id = i.id ";

#[derive(sqlx::FromRow)]
struct Row {
    id: WebInterfaceId,
    device_id: DeviceId,
    label: String,
    url: String,
    open_in: String,
    fingerprint_sha256: Option<String>,
    accepted_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for WebInterface {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Error> {
        let pin = match (row.fingerprint_sha256, row.accepted_at) {
            (Some(text), Some(at)) => Some(Pin {
                fingerprint: rules::fingerprint(&text).map_err(|_| Error::Unreadable)?,
                at,
            }),
            _ => None,
        };
        Ok(Self {
            id: row.id,
            device: row.device_id,
            label: row.label,
            url: row.url,
            open_in: OpenIn::from_stored(&row.open_in).ok_or(Error::Unreadable)?,
            pin,
        })
    }
}

/// The error for a write that broke the foreign key `constraint`: the row it points at is
/// gone.
fn missing_or_database(error: sqlx::Error, constraint: &str, missing: Missing) -> Error {
    match error.as_database_error() {
        Some(failure)
            if failure.is_foreign_key_violation() && failure.constraint() == Some(constraint) =>
        {
            Error::NotFound(missing)
        }
        _ => Error::Database(error),
    }
}

/// The interfaces of the devices in one query, each device's in creation order.
pub(super) async fn for_devices(
    db: impl PgExecutor<'_>,
    devices: &[DeviceId],
) -> Result<Vec<WebInterface>, Error> {
    let sql =
        format!("{SELECT}WHERE i.device_id = ANY($1) ORDER BY i.device_id, i.created_at, i.id");
    let rows: Vec<Row> = sqlx::query_as(&sql).bind(devices).fetch_all(db).await?;
    rows.into_iter().map(WebInterface::try_from).collect()
}

/// The interface; the row stays locked until the connection's transaction ends, so no
/// update or delete can commit between the caller's check and its write.
pub(super) async fn get_locked(
    conn: &mut PgConnection,
    id: WebInterfaceId,
) -> Result<WebInterface, Error> {
    let sql = format!("{SELECT}WHERE i.id = $1 FOR UPDATE OF i");
    let row: Option<Row> = sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?;
    row.ok_or(Error::NotFound(Missing::Interface(id)))?
        .try_into()
}

/// Takes the device's lock until the transaction ends, so two accepts for one device run
/// one after the other.
pub(super) async fn lock_device(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('web_interfaces:' || $1::text, 0))")
        .bind(device)
        .execute(conn)
        .await?;
    Ok(())
}

/// What a new interface is given besides the operator's input.
pub(super) struct Born {
    pub id: WebInterfaceId,
    pub device: DeviceId,
    pub at: DateTime<Utc>,
}

pub(super) async fn insert(
    conn: &mut PgConnection,
    born: Born,
    new: &NewInterface,
) -> Result<WebInterface, Error> {
    let Born { id, device, at } = born;
    sqlx::query(
        "INSERT INTO web_interfaces.interfaces (id, device_id, label, url, open_in, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(device)
    .bind(&new.label)
    .bind(&new.url)
    .bind(new.open_in.as_str())
    .bind(at)
    .execute(conn)
    .await
    .map_err(|error| {
        missing_or_database(error, "interfaces_device_id_fkey", Missing::Device(device))
    })?;
    Ok(WebInterface {
        id,
        device,
        label: new.label.clone(),
        url: new.url.clone(),
        open_in: new.open_in,
        pin: None,
    })
}

/// Replaces label, URL and `open_in`, and drops the pin when the origin changed (D-6), in one
/// transaction. Returns the interface before and after.
pub(super) async fn update(
    conn: &mut PgConnection,
    id: WebInterfaceId,
    new: &NewInterface,
) -> Result<(WebInterface, WebInterface), Error> {
    let mut tx = conn.begin().await?;
    let sql = format!("{SELECT}WHERE i.id = $1 FOR UPDATE OF i");
    let row: Option<Row> = sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    let before = WebInterface::try_from(row.ok_or(Error::NotFound(Missing::Interface(id)))?)?;
    sqlx::query(
        "UPDATE web_interfaces.interfaces SET label = $2, url = $3, open_in = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(&new.label)
    .bind(&new.url)
    .bind(new.open_in.as_str())
    .execute(&mut *tx)
    .await?;
    let origin_changed = !rules::same_origin(&before.url, &new.url);
    if origin_changed {
        sqlx::query("DELETE FROM web_interfaces.pins WHERE interface_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    let after = WebInterface {
        label: new.label.clone(),
        url: new.url.clone(),
        open_in: new.open_in,
        pin: if origin_changed {
            None
        } else {
            before.pin.clone()
        },
        ..before.clone()
    };
    tx.commit().await?;
    Ok((before, after))
}

/// Deletes the interface (its pin goes with it) and returns its device.
pub(super) async fn delete(conn: &mut PgConnection, id: WebInterfaceId) -> Result<DeviceId, Error> {
    let device: Option<DeviceId> = sqlx::query_scalar(
        "DELETE FROM web_interfaces.interfaces WHERE id = $1 RETURNING device_id",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    device.ok_or(Error::NotFound(Missing::Interface(id)))
}

/// Pins the certificate, replacing an earlier pin (D-5).
pub(super) async fn set_pin(
    conn: &mut PgConnection,
    id: WebInterfaceId,
    pin: &Pin,
    operator: OperatorId,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO web_interfaces.pins (interface_id, fingerprint_sha256, accepted_by, accepted_at) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (interface_id) DO UPDATE SET \
             fingerprint_sha256 = excluded.fingerprint_sha256, \
             accepted_by = excluded.accepted_by, accepted_at = excluded.accepted_at",
    )
    .bind(id)
    .bind(pin.fingerprint.as_str())
    .bind(operator)
    .bind(pin.at)
    .execute(conn)
    .await
    .map_err(|error| {
        missing_or_database(error, "pins_interface_id_fkey", Missing::Interface(id))
    })?;
    Ok(())
}

/// Which of these checks have a dismissed suggestion.
pub(super) async fn dismissed_among(
    db: impl PgExecutor<'_>,
    checks: &[CheckId],
) -> Result<HashSet<CheckId>, Error> {
    let ids: Vec<CheckId> = sqlx::query_scalar(
        "SELECT check_id FROM web_interfaces.dismissed WHERE check_id = ANY($1)",
    )
    .bind(checks)
    .fetch_all(db)
    .await?;
    Ok(ids.into_iter().collect())
}

/// Records the dismissal; a second one for the same check changes nothing.
pub(super) async fn insert_dismissed(
    conn: &mut PgConnection,
    check: CheckId,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO web_interfaces.dismissed (check_id, dismissed_at) VALUES ($1, $2) \
         ON CONFLICT (check_id) DO NOTHING",
    )
    .bind(check)
    .bind(now)
    .execute(conn)
    .await
    .map_err(|error| {
        missing_or_database(error, "dismissed_check_id_fkey", Missing::Check(check))
    })?;
    Ok(())
}
