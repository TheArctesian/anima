//! The only SQL for `events.event_log` and `events.audit_log`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Refusal,
    model::{EventSummary, NewAudit, NewEvent},
    retention::{LOGS, Log},
};
use crate::ids::EventId;

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

/// Creates the day partitions of both logs that hold `at`. Each call is its own statement
/// outside any transaction, so the lock it takes on a parent ends at once.
pub(super) async fn create_day_partitions(
    db: &PgPool,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for log in LOGS {
        sqlx::query(ENSURE_DAY_PARTITION)
            .bind(log.table())
            .bind(at)
            .execute(db)
            .await?;
    }
    Ok(())
}

/// Drops the partitions of `log` for UTC days before `first_kept_day` and returns their names.
/// The database refuses a day inside the retention period of `now` or of its own clock.
pub(super) async fn drop_day_partitions_before(
    db: &PgPool,
    log: Log,
    first_kept_day: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("select events.drop_day_partitions_before($1, $2, $3)")
        .bind(log.table())
        .bind(first_kept_day)
        .bind(now)
        .fetch_all(db)
        .await
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
         (id, at, severity, source, device_id, device_name, message) \
         values ($1, $2, $3, $4, $5, $6, $7) returning at",
    )
    .bind(id)
    .bind(at)
    .bind(event.severity.as_str())
    .bind(event.source.as_str())
    .bind(device_id)
    .bind(device_name)
    .bind(&event.message)
    .fetch_one(&mut *conn)
    .await?;
    Ok(EventSummary {
        id,
        at,
        severity: event.severity,
        source: event.source,
        device: event.device,
        message: event.message,
    })
}

/// Inserts the audit entry as given; the caller has already redacted `change`.
pub(super) async fn insert_audit(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    entry: &NewAudit,
) -> Result<(), sqlx::Error> {
    ensure_day_partition(conn, "audit_log", at).await?;
    sqlx::query(
        "insert into events.audit_log \
         (id, at, operator_id, operator_name, action, target_type, target_id, client_addr, change) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
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
    .execute(&mut *conn)
    .await?;
    Ok(())
}
