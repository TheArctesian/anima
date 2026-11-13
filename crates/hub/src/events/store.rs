//! The only SQL for `events.event_log` and `events.audit_log`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    Refusal,
    actor::{ActorKind, TokenKind},
    model::{AuditRecord, EventFilter, EventSummary, NewAudit, NewEvent, Severity, Source},
    page::Position,
    retention::{LOGS, Log},
};
use crate::ids::{DeviceId, EventId};

const ENSURE_DAY_PARTITION: &str = "select events.ensure_day_partition($1, $2)";

/// Creates the UTC-day partition of `parent` that holds `at`, if it does not exist yet (D-4).
/// Inside the caller's transaction.
async fn ensure_day_partition(
    conn: &mut PgConnection,
    parent: &str,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(ENSURE_DAY_PARTITION)
        .bind(parent)
        .bind(at)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// How long a scheduled partition change waits for a lock on a log before it gives up (D-4).
/// Waiting longer would queue every reader and writer of that log behind it; the daily job
/// retries instead.
const SCHEDULED_LOCK_TIMEOUT: &str = "set local lock_timeout = '2s'";

/// A short transaction of its own for one scheduled partition change, with
/// [`SCHEDULED_LOCK_TIMEOUT`]. Never a caller's transaction, so the lock ends at its commit.
async fn scheduled_change(db: &PgPool) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query(SCHEDULED_LOCK_TIMEOUT)
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

/// Creates the day partitions of both logs that hold `at`, each in a short transaction of its
/// own, so the lock it takes on a parent ends at once.
pub(super) async fn create_day_partitions(
    db: &PgPool,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for log in LOGS {
        let mut tx = scheduled_change(db).await?;
        sqlx::query(ENSURE_DAY_PARTITION)
            .bind(log.table())
            .bind(at)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(())
}

/// Drops the partitions of `log` for UTC days before `first_kept_day` and returns their names,
/// in a short transaction of its own. The database refuses a day inside the retention period
/// of `now` or of its own clock.
pub(super) async fn drop_day_partitions_before(
    db: &PgPool,
    log: Log,
    first_kept_day: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, sqlx::Error> {
    let mut tx = scheduled_change(db).await?;
    let dropped = sqlx::query_scalar("select events.drop_day_partitions_before($1, $2, $3)")
        .bind(log.table())
        .bind(first_kept_day)
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(dropped)
}

/// The first reason the hub's login must not serve, or `None` (K2 "Start check", R-023). Reads
/// only the catalog and the privilege functions by OID, so it needs no USAGE on `events`.
pub(super) async fn start_refusal(db: &PgPool) -> Result<Option<Refusal>, sqlx::Error> {
    let code: Option<String> = sqlx::query_scalar(START_REFUSAL).fetch_one(db).await?;
    code.map(|code| match code.as_str() {
        "not_migrated" => Ok(Refusal::NotMigrated),
        "superuser" => Ok(Refusal::Superuser),
        "session_role" => Ok(Refusal::SessionRole),
        "owner" => Ok(Refusal::Owner),
        "server_access" => Ok(Refusal::ServerAccess),
        "can_change_logs" => Ok(Refusal::CanChangeLogs),
        "default_privileges" => Ok(Refusal::DefaultPrivileges),
        _ => Err(sqlx::Error::Decode("unknown start check refusal".into())),
    })
    .transpose()
}

/// One row: the code of the first refusal, in the order of `Refusal`, or null.
///
/// `logs` is both log parents and every partition below them. `roles` is every role the login
/// (`session_user`, not `current_user`, which a session role or `SET ROLE` changes) is a member
/// of, itself included, so a right reached by `SET ROLE`, a grant without INHERIT or ADMIN
/// OPTION counts. Write defaults to `anima_runtime` are 0001's own, for every other domain's
/// tables; `events.ensure_day_partition` strips every grant from a new partition (0502).
/// Default privileges set by any creator role count, not only those of the partitions' owner.
const START_REFUSAL: &str = "\
    with recursive events_schema as ( \
        select n.oid, n.nspowner from pg_catalog.pg_namespace n where n.nspname = 'events' \
    ), logs (oid) as ( \
        select c.oid from pg_catalog.pg_class c \
        join events_schema n on n.oid = c.relnamespace \
        where c.relname in ('event_log', 'audit_log') \
        union \
        select i.inhrelid from pg_catalog.pg_inherits i join logs on i.inhparent = logs.oid \
    ), roles (oid) as ( \
        select r.oid from pg_catalog.pg_roles r \
        where pg_catalog.pg_has_role(session_user, r.oid, 'MEMBER') \
    ) \
    select case \
    when not exists ( \
        select from pg_catalog.pg_class c join events_schema n on n.oid = c.relnamespace \
        where c.relname = 'audit_log' \
    ) then 'not_migrated' \
    when exists (select from pg_catalog.pg_roles r join roles using (oid) where r.rolsuper) \
        then 'superuser' \
    when session_user operator(pg_catalog.<>) current_user then 'session_role' \
    when exists ( \
        select from pg_catalog.pg_database d \
        where d.datname = pg_catalog.current_database() and d.datdba in (select oid from roles) \
    ) or exists (select from events_schema n where n.nspowner in (select oid from roles)) \
    or exists ( \
        select from logs join pg_catalog.pg_class c using (oid) \
        where c.relowner in (select oid from roles) \
    ) then 'owner' \
    when exists ( \
        select from pg_catalog.pg_roles r join roles using (oid) \
        where r.rolname in ( \
            'pg_execute_server_program', 'pg_write_server_files', 'pg_read_server_files', \
            'pg_maintain' \
        ) \
    ) then 'server_access' \
    when exists ( \
        select from roles, logs \
        where pg_catalog.has_any_column_privilege(roles.oid, logs.oid, 'UPDATE, REFERENCES') \
            or pg_catalog.has_table_privilege( \
                roles.oid, logs.oid, 'DELETE, TRUNCATE, TRIGGER, MAINTAIN' \
            ) \
    ) or exists ( \
        select from roles, events_schema n \
        where pg_catalog.has_schema_privilege(roles.oid, n.oid, 'CREATE') \
    ) then 'can_change_logs' \
    when exists ( \
        select from pg_catalog.pg_default_acl d \
        cross join lateral pg_catalog.aclexplode(d.defaclacl) a \
        where d.defaclobjtype = 'r' \
            and (d.defaclnamespace = 0 or d.defaclnamespace in (select oid from events_schema)) \
            and a.privilege_type in \
                ('UPDATE', 'DELETE', 'TRUNCATE', 'TRIGGER', 'REFERENCES', 'MAINTAIN') \
            and ( \
                a.grantee = 0 \
                or a.grantee in ( \
                    select oid from roles where oid is distinct from ( \
                        select r.oid from pg_catalog.pg_roles r \
                        where r.rolname = 'anima_runtime' \
                    ) \
                ) \
            ) \
    ) then 'default_privileges' \
    end";

/// Inserts the event and returns it as stored (`at` at the database's microsecond precision).
pub(super) async fn insert_event(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    event: NewEvent,
) -> Result<EventSummary, sqlx::Error> {
    ensure_day_partition(conn, "event_log", at).await?;
    let id = EventId::from(Uuid::now_v7());
    let (device_id, device_name) = event
        .device
        .as_ref()
        .map(|(id, name)| (*id, name.as_str()))
        .unzip();
    let at = sqlx::query_scalar(
        "insert into events.event_log \
         (id, at, severity, source, device_id, device_name, message, actor_kind) \
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning at",
    )
    .bind(id)
    .bind(at)
    .bind(event.severity.as_str())
    .bind(event.source.as_str())
    .bind(device_id)
    .bind(device_name)
    .bind(&event.message)
    .bind(event.actor.as_str())
    .fetch_one(&mut *conn)
    .await?;
    Ok(EventSummary {
        id,
        at,
        severity: event.severity,
        source: event.source,
        device: event.device,
        message: event.message,
        actor: Some(event.actor),
    })
}

/// Inserts the audit entry as given; the caller has already redacted `change`. The actor's
/// kind, token kind and key name go in their own columns; never the token (LOG-018).
pub(super) async fn insert_audit(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    entry: &NewAudit,
) -> Result<(), sqlx::Error> {
    ensure_day_partition(conn, "audit_log", at).await?;
    let actor = &entry.operator.actor;
    sqlx::query(
        "insert into events.audit_log \
         (id, at, operator_id, operator_name, action, target_type, target_id, client_addr, \
         change, actor_kind, token_kind, api_key_name) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(Uuid::now_v7())
    .bind(at)
    .bind(entry.operator.id)
    .bind(&entry.operator.name)
    .bind(entry.action.as_str())
    .bind(entry.target.kind())
    .bind(entry.target.id())
    .bind(entry.client.0)
    .bind(&entry.change)
    .bind(actor.kind().as_str())
    .bind(actor.token().map(TokenKind::as_str))
    .bind(actor.api_key_name())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// A stored `event_log` row.
#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    at: DateTime<Utc>,
    severity: String,
    source: String,
    device_id: Option<DeviceId>,
    device_name: Option<String>,
    message: String,
    actor_kind: Option<String>,
}

impl TryFrom<EventRow> for EventSummary {
    type Error = sqlx::Error;

    fn try_from(row: EventRow) -> Result<Self, Self::Error> {
        let unknown = |column: &str| sqlx::Error::Decode(format!("unknown event {column}").into());
        Ok(Self {
            id: row.id.into(),
            at: row.at,
            severity: Severity::from_stored(&row.severity).ok_or_else(|| unknown("severity"))?,
            source: Source::from_stored(&row.source).ok_or_else(|| unknown("source"))?,
            device: row.device_id.zip(row.device_name),
            message: row.message,
            actor: stored_kind(row.actor_kind.as_deref(), ActorKind::from_stored)?,
        })
    }
}

/// Up to `limit` events that pass `filter`, newest first, after `after` when it is given.
pub(super) async fn list_events(
    db: &PgPool,
    filter: &EventFilter,
    after: Option<Position>,
    limit: i64,
) -> Result<Vec<EventSummary>, sqlx::Error> {
    let severities: Option<Vec<&str>> = filter
        .severities
        .as_ref()
        .map(|all| all.iter().map(|severity| severity.as_str()).collect());
    let sources: Option<Vec<&str>> = filter
        .sources
        .as_ref()
        .map(|all| all.iter().map(|source| source.as_str()).collect());
    let rows: Vec<EventRow> = sqlx::query_as(
        "select id, at, severity, source, device_id, device_name, message, actor_kind \
         from events.event_log \
         where ($1::text[] is null or severity = any($1)) \
           and ($2::text[] is null or source = any($2)) \
           and ($3::uuid is null or device_id = $3) \
           and ($4::timestamptz is null or at >= $4) \
           and ($5::timestamptz is null or (at, id) < ($5, $6)) \
         order by at desc, id desc limit $7",
    )
    .bind(severities)
    .bind(sources)
    .bind(filter.device)
    .bind(filter.since)
    .bind(after.map(|position| position.at))
    .bind(after.map(|position| position.id))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(EventSummary::try_from).collect()
}

/// A stored `audit_log` row.
#[derive(sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    at: DateTime<Utc>,
    operator_name: String,
    action: String,
    target_type: String,
    target_id: String,
    client_addr: std::net::IpAddr,
    change: serde_json::Value,
    actor_kind: Option<String>,
    token_kind: Option<String>,
    api_key_name: Option<String>,
}

/// The kind stored as `text`, read with `parse`; `None` for a row written before the column
/// existed, an error for text this hub does not know.
fn stored_kind<T>(
    text: Option<&str>,
    parse: impl Fn(&str) -> Option<T>,
) -> Result<Option<T>, sqlx::Error> {
    text.map(|text| {
        parse(text).ok_or_else(|| sqlx::Error::Decode("unknown actor or token kind".into()))
    })
    .transpose()
}

/// Up to `limit` audit entries of `actor` (every kind when `None`), newest first, after
/// `after` when it is given. An entry with no recorded kind matches only `None` (LOG-020).
pub(super) async fn list_audit(
    db: &PgPool,
    actor: Option<ActorKind>,
    after: Option<Position>,
    limit: i64,
) -> Result<Vec<AuditRecord>, sqlx::Error> {
    let rows: Vec<AuditRow> = sqlx::query_as(
        "select id, at, operator_name, action, target_type, target_id, client_addr, change, \
         actor_kind, token_kind, api_key_name \
         from events.audit_log \
         where ($1::text is null or actor_kind = $1) \
           and ($2::timestamptz is null or (at, id) < ($2, $3)) \
         order by at desc, id desc limit $4",
    )
    .bind(actor.map(ActorKind::as_str))
    .bind(after.map(|position| position.at))
    .bind(after.map(|position| position.id))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(AuditRecord::try_from).collect()
}

impl TryFrom<AuditRow> for AuditRecord {
    type Error = sqlx::Error;

    fn try_from(row: AuditRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            at: row.at,
            operator_name: row.operator_name,
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            client: row.client_addr,
            change: row.change,
            actor_kind: stored_kind(row.actor_kind.as_deref(), ActorKind::from_stored)?,
            token_kind: stored_kind(row.token_kind.as_deref(), TokenKind::from_stored)?,
            api_key_name: row.api_key_name,
        })
    }
}
