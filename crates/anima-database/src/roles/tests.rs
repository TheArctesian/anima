//! The roles step against a real server, and against what `deploy/postgres/roles.sql` makes.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use sqlx::Connection as _;
use sqlx::postgres::PgConnection;

use super::{login_statement, prepare};
use crate::scratch::Database;
use crate::{Layout, Role, urls};

/// What a role may do and who may connect where, as one text to compare.
const RIGHTS: &str = "
    select r.rolname, r.rolcanlogin, r.rolsuper, r.rolcreatedb, r.rolcreaterole,
           r.rolbypassrls, r.rolreplication, r.rolinherit,
           (select string_agg(g.rolname, ',' order by g.rolname)
              from pg_auth_members m join pg_roles g on g.oid = m.roleid
             where m.member = r.oid)
      from pg_roles r
     where r.rolname in ('anima_owner', 'anima_hub', 'anima_runtime')
     order by 1;
    select d.datname, pg_get_userbyid(d.datdba), d.datacl
      from pg_database d
     where d.datname in ('anima', 'postgres', 'template1')
     order by 1;";

/// Signs in with the URL in `role`'s file exactly as the hub does (`connect(url.trim())`) and
/// runs one statement; `Err` holds the SQLSTATE.
fn as_role(layout: &Layout, role: Role, statement: &str) -> Result<(), String> {
    let url = fs::read_to_string(layout.url_file(role)).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut session = PgConnection::connect(url.trim()).await?;
            sqlx::raw_sql(statement).execute(&mut session).await?;
            session.close().await
        })
        .map_err(|error: sqlx::Error| match error.as_database_error() {
            Some(database) => database.code().unwrap_or_default().into_owned(),
            None => format!("no SQLSTATE: {error}"),
        })
}

fn rights(database: &Database) -> String {
    let output = database
        .psql("postgres")
        .args(["-c", RIGHTS])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn db_003_r_016_the_roles_are_the_roles_of_the_docker_installation() {
    let bundled = Database::new();
    let _bundled_server = bundled.start();
    prepare(&bundled.layout).unwrap();

    let docker = Database::new();
    let _docker_server = docker.start();
    let secrets = tempfile::tempdir().unwrap();
    for name in ["owner", "hub"] {
        fs::write(secrets.path().join(name), "a-test-password").unwrap();
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/postgres/roles.sql");
    let ran = docker
        .psql("postgres")
        .env("PATH", "/bin:/usr/bin")
        .env("ANIMA_OWNER_PASSWORD_FILE", secrets.path().join("owner"))
        .env("ANIMA_HUB_PASSWORD_FILE", secrets.path().join("hub"))
        .arg("-f")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    assert_eq!(rights(&bundled), rights(&docker));
    assert!(rights(&bundled).contains("anima_hub|t|f|f|f|f|f|t|anima_runtime"));
}

#[test]
fn db_003_the_hub_login_owns_nothing_and_reaches_only_its_database() {
    let database = Database::new();
    let _server = database.start();
    prepare(&database.layout).unwrap();
    let layout = &database.layout;

    assert_eq!(as_role(layout, Role::Hub, "select 1"), Ok(()));
    assert_eq!(
        as_role(layout, Role::Hub, "create table public.mine (n int)"),
        Err("42501".to_owned()),
        "insufficient privilege"
    );
    assert_eq!(
        as_role(layout, Role::Hub, "create schema mine"),
        Err("42501".to_owned())
    );
    assert_eq!(as_role(layout, Role::Owner, "create schema theirs"), Ok(()));
    assert_eq!(
        as_role(layout, Role::Owner, "create table theirs.rows (n int)"),
        Ok(())
    );

    // The hub's URL with the `postgres` database in place of its own.
    let elsewhere = fs::read_to_string(layout.url_file(Role::Hub))
        .unwrap()
        .replace("/anima?", "/postgres?");
    fs::write(layout.url_file(Role::Hub), elsewhere).unwrap();
    assert_eq!(
        as_role(layout, Role::Hub, "select 1"),
        Err("42501".to_owned()),
        "no CONNECT on the postgres database"
    );
}

#[test]
fn db_002_log_007_the_passwords_are_in_private_files_and_not_in_the_log() {
    let database = Database::new();
    let _server = database.start();

    prepare(&database.layout).unwrap();

    let mut names = Vec::new();
    for entry in fs::read_dir(database.layout.urls_folder()).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            entry.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        names.push(entry.file_name().into_string().unwrap());
    }
    names.sort();
    assert_eq!(names, ["hub.url", "owner.url"]);

    let log = fs::read_to_string(database.layout.server_log()).unwrap();
    for role in [Role::Owner, Role::Hub] {
        let password = urls::password(&database.layout, role).unwrap().unwrap();
        assert!(!log.contains(password.expose()), "the log holds a password");
    }
}

#[test]
fn a_second_run_after_a_crash_gives_logins_that_work() {
    let database = Database::new();
    let _server = database.start();
    prepare(&database.layout).unwrap();
    as_role(&database.layout, Role::Owner, "create schema kept").unwrap();
    let first = urls::password(&database.layout, Role::Hub)
        .unwrap()
        .unwrap();
    // A crash after the roles were made and before the hub's file was written.
    fs::remove_file(database.layout.url_file(Role::Hub)).unwrap();

    prepare(&database.layout).unwrap();

    assert_eq!(as_role(&database.layout, Role::Hub, "select 1"), Ok(()));
    assert_eq!(
        as_role(
            &database.layout,
            Role::Owner,
            "create table kept.rows (n int)"
        ),
        Ok(()),
        "the owner's schema from before the second run is still the owner's"
    );
    let second = urls::password(&database.layout, Role::Hub)
        .unwrap()
        .unwrap();
    assert_ne!(first.expose(), second.expose());
}

#[test]
fn log_007_a_login_statement_holds_the_verifier_and_never_a_password() {
    let password = crate::Password::from_bytes(&[5; 24]);
    let verifier = password.scram_verifier(&[2; 16]);

    let made = login_statement("anima_hub", false, "anima_runtime", &verifier);
    let changed = login_statement("anima_owner", true, "", &verifier);

    assert_eq!(
        made,
        format!(
            "create role anima_hub login nosuperuser nocreatedb nocreaterole nobypassrls \
             noreplication in role anima_runtime password '{verifier}'"
        )
    );
    assert_eq!(
        changed,
        format!(
            "alter role anima_owner login nosuperuser nocreatedb nocreaterole nobypassrls \
             noreplication password '{verifier}'"
        )
    );
    assert!(!made.contains(password.expose()) && !changed.contains(password.expose()));
}

#[test]
fn log_007_what_the_server_stores_is_what_it_was_sent_and_the_password_signs_in() {
    let database = Database::new();
    let _server = database.start();
    prepare(&database.layout).unwrap();

    let stored = database
        .sql("select rolpassword from pg_authid where rolname = 'anima_hub'")
        .unwrap();
    let password = urls::password(&database.layout, Role::Hub)
        .unwrap()
        .unwrap();

    assert!(stored.starts_with("SCRAM-SHA-256$4096:"), "{stored}");
    assert!(!stored.contains(password.expose()));
    assert_eq!(as_role(&database.layout, Role::Hub, "select 1"), Ok(()));
}

#[test]
fn a_database_that_another_role_owns_is_given_to_the_owner() {
    let database = Database::new();
    let _server = database.start();
    database.sql("create database anima").unwrap();

    prepare(&database.layout).unwrap();

    assert_eq!(
        database
            .sql("select pg_get_userbyid(datdba) from pg_database where datname = 'anima'")
            .unwrap(),
        "anima_owner"
    );
}
