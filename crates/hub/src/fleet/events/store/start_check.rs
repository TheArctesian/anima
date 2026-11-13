//! The start check of the hub's login against the logs (K2 "Start check", R-023).

use sqlx::PgPool;

use crate::fleet::events::Refusal;

/// The first reason the hub's login must not serve, or `None` (K2 "Start check", R-023). Reads
/// only the catalog and the privilege functions by OID, so it needs no USAGE on `events`.
pub(in crate::fleet::events) async fn start_refusal(
    db: &PgPool,
) -> Result<Option<Refusal>, sqlx::Error> {
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
