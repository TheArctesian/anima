//! The only SQL for `events.event_log` and `events.audit_log`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
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

/// What the connected role could do beyond `anima_runtime` (K2 "Roles"). Each field counts
/// every role the connected role is a member of, so a right reached by `SET ROLE`, through a
/// grant without INHERIT or through ADMIN OPTION counts too.
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent facts read from the catalog; events::check_runtime_role turns them into one refusal"
)]
pub(super) struct RuntimeRole {
    /// It is, or may become, a superuser.
    pub(super) superuser: bool,
    /// It may become the owner of a log table or of one of its partitions.
    pub(super) owns_logs: bool,
    /// It may become the owner of schema `events`, who may drop any table in it.
    pub(super) owns_events_schema: bool,
    /// It may update (any column), delete, truncate or add a trigger to a log table or one of
    /// its partitions, or create objects in schema `events`.
    pub(super) can_change_logs: bool,
}

/// Reads what the connected role could do to the logs, or `None` if the `events` schema has
/// not been migrated. Reads only the catalog and the privilege functions by OID, so it needs
/// no USAGE on `events`.
pub(super) async fn runtime_role(db: &PgPool) -> Result<Option<RuntimeRole>, sqlx::Error> {
    let row: Option<(bool, bool, bool, bool)> =
        sqlx::query_as(RUNTIME_ROLE).fetch_optional(db).await?;
    Ok(row.map(
        |(superuser, owns_logs, owns_events_schema, can_change_logs)| RuntimeRole {
            superuser,
            owns_logs,
            owns_events_schema,
            can_change_logs,
        },
    ))
}

/// One row when `events.audit_log` exists. `logs` is both parents and every partition below
/// them; `roles` is every role the connected role is a member of, itself included.
const RUNTIME_ROLE: &str = "\
    with recursive logs (oid) as ( \
        select c.oid from pg_catalog.pg_class c \
        join pg_catalog.pg_namespace n on n.oid = c.relnamespace \
        where n.nspname = 'events' and c.relname in ('event_log', 'audit_log') \
        union \
        select i.inhrelid from pg_catalog.pg_inherits i join logs on i.inhparent = logs.oid \
    ), roles (oid) as ( \
        select r.oid from pg_catalog.pg_roles r \
        where pg_catalog.pg_has_role(current_user, r.oid, 'MEMBER') \
    ) \
    select \
        exists (select from pg_catalog.pg_roles r join roles using (oid) where r.rolsuper), \
        exists ( \
            select from logs join pg_catalog.pg_class c using (oid) \
            where c.relowner in (select oid from roles) \
        ), \
        n.nspowner in (select oid from roles), \
        exists ( \
            select from roles, logs \
            where pg_catalog.has_any_column_privilege(roles.oid, logs.oid, 'UPDATE') \
                or pg_catalog.has_table_privilege(roles.oid, logs.oid, 'DELETE, TRUNCATE, TRIGGER') \
        ) or exists ( \
            select from roles \
            where pg_catalog.has_schema_privilege(roles.oid, n.oid, 'CREATE') \
        ) \
    from pg_catalog.pg_class a \
    join pg_catalog.pg_namespace n on n.oid = a.relnamespace \
    where n.nspname = 'events' and a.relname = 'audit_log'";

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
