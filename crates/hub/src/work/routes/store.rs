//! All SQL on schema `routes`: I/O only. Ids and time come in as parameters. Writes take the
//! caller's connection so the caller adds its audit entry in the same transaction.

use std::net::{IpAddr, SocketAddr};

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{
    Error,
    model::{CertificateState, Route, Traffic},
    names::RouteName,
    proof::Proof,
};
use crate::ids::{ApplicationId, DeviceId, RouteId};

const SELECT: &str = "SELECT r.id, r.name, r.application_id, r.device_id, r.port, r.address, \
                             r.token, r.proof, r.failed_reads, r.machine_consent, r.checked_at, \
                             r.issue_failures, r.next_issue_at, r.created_at, \
                             c.not_after, c.stored_at, \
                             t.requests, t.bytes_in, t.bytes_out, t.refused \
                      FROM routes.route r \
                      LEFT JOIN routes.certificate c ON c.route_id = r.id \
                      LEFT JOIN routes.traffic t ON t.route_id = r.id ";

#[derive(sqlx::FromRow)]
struct Row {
    id: RouteId,
    name: String,
    application_id: ApplicationId,
    device_id: DeviceId,
    port: i32,
    address: Option<IpAddr>,
    token: String,
    proof: String,
    failed_reads: i32,
    machine_consent: bool,
    checked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    not_after: Option<DateTime<Utc>>,
    stored_at: Option<DateTime<Utc>>,
    requests: Option<i64>,
    bytes_in: Option<i64>,
    bytes_out: Option<i64>,
    refused: Option<i64>,
}

fn count(value: Option<i64>) -> u64 {
    value
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0)
}

impl TryFrom<Row> for Route {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Error> {
        let port = u16::try_from(row.port).map_err(|_| Error::Unreadable)?;
        Ok(Self {
            id: row.id,
            name: RouteName::stored(row.name),
            application: row.application_id,
            device: row.device_id,
            port,
            address: row.address.map(|ip| SocketAddr::new(ip, port)),
            token: row.token,
            proof: Proof::from_stored(&row.proof).ok_or(Error::Unreadable)?,
            failed_reads: u32::try_from(row.failed_reads).map_err(|_| Error::Unreadable)?,
            machine_consent: row.machine_consent,
            checked_at: row.checked_at,
            certificate: row
                .not_after
                .zip(row.stored_at)
                .map(|(not_after, stored_at)| CertificateState {
                    not_after,
                    stored_at,
                }),
            traffic: Traffic {
                requests: count(row.requests),
                bytes_in: count(row.bytes_in),
                bytes_out: count(row.bytes_out),
                refused: count(row.refused),
            },
            created_at: row.created_at,
        })
    }
}

/// The routes, of one application or of all, by name.
pub(super) async fn list(
    db: impl PgExecutor<'_>,
    application: Option<ApplicationId>,
) -> Result<Vec<Route>, Error> {
    let sql = format!("{SELECT}WHERE $1::uuid IS NULL OR r.application_id = $1 ORDER BY r.name");
    let rows: Vec<Row> = sqlx::query_as(&sql).bind(application).fetch_all(db).await?;
    rows.into_iter().map(Route::try_from).collect()
}

/// One route.
pub(super) async fn get(db: impl PgExecutor<'_>, id: RouteId) -> Result<Option<Route>, Error> {
    let sql = format!("{SELECT}WHERE r.id = $1");
    let row: Option<Row> = sqlx::query_as(&sql).bind(id).fetch_optional(db).await?;
    row.map(Route::try_from).transpose()
}

/// Inserts a new route; [`Error::NameTaken`] when a route already has its name.
pub(super) async fn insert(conn: &mut PgConnection, route: &Route) -> Result<(), Error> {
    let result = sqlx::query(
        "INSERT INTO routes.route (id, name, application_id, device_id, port, address, token, \
                                   created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(route.id)
    .bind(route.name.as_str())
    .bind(route.application)
    .bind(route.device)
    .bind(i32::from(route.port))
    .bind(route.address.map(|address| address.ip()))
    .bind(&route.token)
    .bind(route.created_at)
    .execute(conn)
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(error)
            if error
                .as_database_error()
                .is_some_and(sqlx::error::DatabaseError::is_unique_violation) =>
        {
            Err(Error::NameTaken)
        }
        Err(error) => Err(Error::Database(error)),
    }
}

/// Deletes a route with its certificate and its counts; the device it had, or `None`.
pub(super) async fn delete(
    conn: &mut PgConnection,
    id: RouteId,
) -> Result<Option<(DeviceId, String)>, Error> {
    Ok(
        sqlx::query_as("DELETE FROM routes.route WHERE id = $1 RETURNING device_id, name")
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Writes the state of the proof after one read.
pub(super) async fn set_proof(
    conn: &mut PgConnection,
    id: RouteId,
    (proof, failed_reads): (Proof, u32),
    at: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE routes.route SET proof = $2, failed_reads = $3, checked_at = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(proof.as_str())
    .bind(i32::try_from(failed_reads).unwrap_or(i32::MAX))
    .bind(at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The failed orders in a row of a route and when it may ask again.
pub(super) async fn issue_state(
    db: impl PgExecutor<'_>,
    id: RouteId,
) -> Result<(u32, Option<DateTime<Utc>>), Error> {
    let (failures, next): (i32, Option<DateTime<Utc>>) =
        sqlx::query_as("SELECT issue_failures, next_issue_at FROM routes.route WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?;
    Ok((u32::try_from(failures).unwrap_or(0), next))
}

/// Records a failed order.
pub(super) async fn issue_failed(
    db: impl PgExecutor<'_>,
    id: RouteId,
    failures: u32,
    next: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("UPDATE routes.route SET issue_failures = $2, next_issue_at = $3 WHERE id = $1")
        .bind(id)
        .bind(i32::try_from(failures).unwrap_or(i32::MAX))
        .bind(next)
        .execute(db)
        .await?;
    Ok(())
}

/// A certificate with its sealed key, as stored.
pub(super) struct StoredCertificate {
    pub chain_pem: String,
    pub sealed_key: Vec<u8>,
    pub not_after: DateTime<Utc>,
}

/// Replaces the certificate of a route and clears its failed orders.
pub(super) async fn store_certificate(
    conn: &mut PgConnection,
    id: RouteId,
    certificate: &StoredCertificate,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO routes.certificate (route_id, chain_pem, sealed_key, not_after, stored_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (route_id) DO UPDATE SET chain_pem = $2, sealed_key = $3, not_after = $4, \
                                              stored_at = $5",
    )
    .bind(id)
    .bind(&certificate.chain_pem)
    .bind(&certificate.sealed_key)
    .bind(certificate.not_after)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE routes.route SET issue_failures = 0, next_issue_at = NULL WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// The certificate of a route with its sealed key.
pub(super) async fn certificate(
    db: impl PgExecutor<'_>,
    id: RouteId,
) -> Result<Option<StoredCertificate>, Error> {
    let row: Option<(String, Vec<u8>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT chain_pem, sealed_key, not_after FROM routes.certificate WHERE route_id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(
        row.map(|(chain_pem, sealed_key, not_after)| StoredCertificate {
            chain_pem,
            sealed_key,
            not_after,
        }),
    )
}

/// Adds what the proxy counted to the counts of a route.
pub(super) async fn add_traffic(
    db: impl PgExecutor<'_>,
    id: RouteId,
    traffic: Traffic,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let big = |value: u64| i64::try_from(value).unwrap_or(i64::MAX);
    sqlx::query(
        "INSERT INTO routes.traffic (route_id, requests, bytes_in, bytes_out, refused, updated_at) \
         SELECT id, $2, $3, $4, $5, $6 FROM routes.route WHERE id = $1 \
         ON CONFLICT (route_id) DO UPDATE SET \
             requests = routes.traffic.requests + $2, bytes_in = routes.traffic.bytes_in + $3, \
             bytes_out = routes.traffic.bytes_out + $4, refused = routes.traffic.refused + $5, \
             updated_at = $6",
    )
    .bind(id)
    .bind(big(traffic.requests))
    .bind(big(traffic.bytes_in))
    .bind(big(traffic.bytes_out))
    .bind(big(traffic.refused))
    .bind(now)
    .execute(db)
    .await?;
    Ok(())
}
