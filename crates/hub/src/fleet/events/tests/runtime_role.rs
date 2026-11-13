//! The start check refuses a hub login that could change the logs (K2 "Start check", R-016,
//! R-023, LOG-006). Every case logs in as a real LOGIN role.

use sqlx::{PgConnection, PgPool};

use super::{login_pool, privileges::execute, test_role};
use crate::fleet::events::{Refusal, RoleError, check_runtime_role};

/// Runs `sql`, which may hold several statements, with `{role}` replaced by `role` and `{db}`
/// by the test database's name.
async fn run_for(conn: &mut PgConnection, sql: &str, role: &str) {
    let db: String = sqlx::query_scalar("select current_database()::text")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let sql = sql.replace("{role}", role).replace("{db}", &db);
    sqlx::raw_sql(&sql).execute(conn).await.unwrap();
}

/// Creates both logs' partitions for 2030-05-05 and takes USAGE on schema `events` away from
/// `anima_runtime`, so the check must work from the catalog alone.
async fn partitions_without_schema_usage(conn: &mut PgConnection) {
    for log in ["event_log", "audit_log"] {
        execute(
            conn,
            &format!("select events.ensure_day_partition('{log}', '2030-05-05 12:00+00')"),
        )
        .await;
    }
    execute(conn, "revoke usage on schema events from anima_runtime").await;
}

/// The refusal for each `(setup, teardown)`: a new LOGIN role `{role}` in `anima_runtime` gets
/// `setup`, logs in and runs the start check, then `teardown` and the role are removed.
async fn outcomes(pool: &PgPool, cases: &[(&str, &str)]) -> Vec<Option<Refusal>> {
    let mut conn = pool.acquire().await.unwrap();
    partitions_without_schema_usage(&mut conn).await;
    let mut found = Vec::new();
    for (setup, teardown) in cases {
        let role = test_role("hub");
        execute(
            &mut conn,
            &format!("create role {role} login in role anima_runtime"),
        )
        .await;
        run_for(&mut conn, setup, &role).await;
        let login = login_pool(pool, &role).await;
        let outcome = check_runtime_role(&login).await;
        login.close().await;
        run_for(&mut conn, teardown, &role).await;
        execute(&mut conn, &format!("drop owned by {role}")).await;
        execute(&mut conn, &format!("drop role {role}")).await;
        found.push(match outcome {
            Ok(()) => None,
            Err(RoleError::Refused(refusal)) => Some(refusal),
            Err(error) => panic!("{setup}: {error:?}"),
        });
    }
    found
}

/// Asserts that every case is refused with `expected`.
async fn assert_refused(pool: &PgPool, cases: &[(&str, &str)], expected: Refusal) {
    let found = outcomes(pool, cases).await;
    for ((setup, _), outcome) in cases.iter().zip(found) {
        assert_eq!(outcome, Some(expected), "{setup}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_superuser(pool: PgPool) {
    let outcome = check_runtime_role(&pool).await;

    assert!(
        matches!(outcome, Err(RoleError::Refused(Refusal::Superuser))),
        "{outcome:?}"
    );
}

/// The login's rights count, not the session role's: a superuser login whose session runs as
/// `anima_runtime` is refused as a superuser (R-023).
#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_refuses_a_superuser_login_behind_a_session_role(pool: PgPool) {
    let session = super::pool_as(&pool, "anima_runtime").await;
    let outcome = check_runtime_role(&session).await;
    session.close().await;

    assert!(
        matches!(outcome, Err(RoleError::Refused(Refusal::Superuser))),
        "{outcome:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_login_that_can_become_a_superuser(pool: PgPool) {
    let cases = [(
        "do $$ begin execute format('grant %I to {role}', current_user); end $$",
        "",
    )];

    assert_refused(&pool, &cases, Refusal::Superuser).await;
}

/// A session role hides the login's own rights from `current_user`; `SET ROLE` or `RESET
/// ROLE` brings them back (R-023).
#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_refuses_a_login_whose_session_runs_as_another_role(pool: PgPool) {
    let cases = [("alter role {role} set role = 'anima_runtime'", "")];
    assert_refused(&pool, &cases, Refusal::SessionRole).await;

    let mut conn = pool.acquire().await.unwrap();
    let role = test_role("hub");
    execute(
        &mut conn,
        &format!("create role {role} login in role anima_runtime"),
    )
    .await;
    let options = (*pool.connect_options())
        .clone()
        .username(&role)
        .options([("role", "anima_runtime")]);
    let session = super::connect(options).await;
    let outcome = check_runtime_role(&session).await;
    session.close().await;
    execute(&mut conn, &format!("drop role {role}")).await;
    assert!(
        matches!(outcome, Err(RoleError::Refused(Refusal::SessionRole))),
        "{outcome:?}"
    );
}

/// Owning the database, schema `events`, a log parent or a partition, or being a member of
/// such an owner, is refused.
#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_refuses_an_owner_of_the_database_schema_or_logs(pool: PgPool) {
    let mut cases = vec![
        (
            "alter database \"{db}\" owner to {role}".to_owned(),
            "alter database \"{db}\" owner to current_user".to_owned(),
        ),
        (
            "create role {role}_o nologin; grant {role}_o to {role}; \
             alter schema events owner to {role}_o"
                .to_owned(),
            "alter schema events owner to current_user; drop role {role}_o".to_owned(),
        ),
    ];
    for table in [
        "audit_log",
        "event_log",
        "audit_log_20300505",
        "event_log_20300505",
    ] {
        cases.push((
            format!("alter table events.{table} owner to {{role}}"),
            format!("alter table events.{table} owner to current_user"),
        ));
        cases.push((
            format!(
                "create role {{role}}_o nologin; grant {{role}}_o to {{role}}; \
                 alter table events.{table} owner to {{role}}_o"
            ),
            format!("alter table events.{table} owner to current_user; drop role {{role}}_o"),
        ));
    }
    let cases: Vec<(&str, &str)> = cases
        .iter()
        .map(|(setup, teardown)| (setup.as_str(), teardown.as_str()))
        .collect();

    assert_refused(&pool, &cases, Refusal::Owner).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_refuses_members_of_server_file_program_and_maintain_roles(pool: PgPool) {
    let cases = [
        ("grant pg_execute_server_program to {role}", ""),
        ("grant pg_write_server_files to {role}", ""),
        ("grant pg_read_server_files to {role}", ""),
        ("grant pg_maintain to {role}", ""),
    ];

    assert_refused(&pool, &cases, Refusal::ServerAccess).await;
}

/// Each case gives the login one right that could change, remove, lock or pin log entries.
const CHANGE_RIGHTS: [(&str, &str); 14] = [
    ("grant pg_write_all_data to {role}", ""),
    ("grant update on events.audit_log_20300505 to {role}", ""),
    (
        "grant update (operator_name) on events.audit_log_20300505 to {role}",
        "",
    ),
    ("grant delete on events.event_log_20300505 to {role}", ""),
    ("grant truncate on events.audit_log to {role}", ""),
    ("grant trigger on events.event_log to {role}", ""),
    ("grant maintain on events.audit_log_20300505 to {role}", ""),
    ("grant references on events.audit_log to {role}", ""),
    (
        "grant references (id) on events.event_log_20300505 to {role}",
        "",
    ),
    ("grant create on schema events to {role}", ""),
    (
        "grant delete on events.audit_log_20300505 to public",
        "revoke delete on events.audit_log_20300505 from public",
    ),
    (
        "create role {role}_w nologin; \
         grant update on events.event_log_20300505 to {role}_w; \
         grant {role}_w to {role} with inherit false",
        "drop owned by {role}_w; drop role {role}_w",
    ),
    (
        "create role {role}_w nologin; \
         grant pg_write_all_data to {role}_w; \
         grant {role}_w to {role} with inherit false, set true",
        "drop role {role}_w",
    ),
    (
        "create role {role}_w nologin; \
         grant truncate on events.audit_log to {role}_w; \
         grant {role}_w to {role} with admin true, inherit false, set false",
        "drop owned by {role}_w; drop role {role}_w",
    ),
];

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_login_that_can_change_log_entries(pool: PgPool) {
    assert_refused(&pool, &CHANGE_RIGHTS, Refusal::CanChangeLogs).await;
}

/// Default privileges that would give the login, a role it may become, or PUBLIC write rights
/// on tables created later, globally or in schema `events` (R-023).
const DEFAULT_RIGHTS: [(&str, &str); 5] = [
    (
        "alter default privileges grant update on tables to {role}",
        "alter default privileges revoke update on tables from {role}",
    ),
    (
        "alter default privileges grant delete on tables to public",
        "alter default privileges revoke delete on tables from public",
    ),
    (
        "alter default privileges in schema events grant truncate on tables to public",
        "alter default privileges in schema events revoke truncate on tables from public",
    ),
    (
        "create role {role}_w nologin; grant {role}_w to {role}; \
         alter default privileges in schema events grant maintain, references on tables \
         to {role}_w",
        "alter default privileges in schema events revoke maintain, references on tables \
         from {role}_w; drop role {role}_w",
    ),
    (
        "alter default privileges grant trigger on tables to {role}",
        "alter default privileges revoke trigger on tables from {role}",
    ),
];

#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_refuses_write_rights_through_default_privileges(pool: PgPool) {
    assert_refused(&pool, &DEFAULT_RIGHTS, Refusal::DefaultPrivileges).await;
}

/// A plain member passes, also with read-only and monitoring roles, without USAGE on `events`
/// and beside 0001's own default privileges for `anima_runtime` and defaults outside `events`
/// and for sequences.
#[sqlx::test(migrations = "./migrations")]
async fn r_023_start_check_passes_a_plain_member_of_anima_runtime(pool: PgPool) {
    let cases = [
        ("", ""),
        (
            "grant pg_read_all_data, pg_monitor, pg_signal_backend to {role}",
            "",
        ),
        (
            "create schema {role}_s; \
             alter default privileges in schema {role}_s grant update on tables to {role}; \
             alter default privileges grant update on sequences to {role}",
            "alter default privileges in schema {role}_s revoke update on tables from {role}; \
             alter default privileges revoke update on sequences from {role}; \
             drop schema {role}_s",
        ),
    ];

    let found = outcomes(&pool, &cases).await;

    assert_eq!(found, [None, None, None]);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_start_check_refuses_a_database_without_the_events_schema(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    execute(&mut conn, "drop schema events cascade").await;

    let outcome = check_runtime_role(&pool).await;

    assert!(
        matches!(outcome, Err(RoleError::Refused(Refusal::NotMigrated))),
        "{outcome:?}"
    );
}

#[test]
fn api_010_start_check_errors_are_plain_sentences() {
    let refusals = [
        Refusal::Superuser,
        Refusal::SessionRole,
        Refusal::Owner,
        Refusal::ServerAccess,
        Refusal::CanChangeLogs,
        Refusal::DefaultPrivileges,
    ];
    for refusal in refusals {
        let message = RoleError::from(refusal).to_string();
        assert!(message.starts_with("The hub's database login"), "{message}");
        assert!(message.ends_with('.'), "{message}");
    }
    for message in [
        RoleError::from(Refusal::NotMigrated).to_string(),
        RoleError::Database(sqlx::Error::PoolTimedOut).to_string(),
    ] {
        assert!(message.starts_with("The "), "{message}");
        assert!(message.ends_with('.'), "{message}");
    }
}
