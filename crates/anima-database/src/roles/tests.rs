//! The roles step against a real server, and against what `deploy/postgres/roles.sql` makes.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use sqlx::Connection as _;
use sqlx::postgres::PgConnection;

use super::prepare;
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
    prepare(&bundled.layout, &bundled.password).unwrap();

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
    prepare(&database.layout, &database.password).unwrap();
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
fn db_002_log_007_the_passwords_are_in_two_private_files_and_nowhere_else() {
    let database = Database::new();
    let _server = database.start();

    prepare(&database.layout, &database.password).unwrap();

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
    assert!(!log.contains(database.password.expose()));
}

#[test]
fn a_second_run_after_a_crash_gives_logins_that_work() {
    let database = Database::new();
    let _server = database.start();
    prepare(&database.layout, &database.password).unwrap();
    as_role(&database.layout, Role::Owner, "create schema kept").unwrap();
    let first = urls::password(&database.layout, Role::Hub)
        .unwrap()
        .unwrap();
    // A crash after the roles were made and before the hub's file was written.
    fs::remove_file(database.layout.url_file(Role::Hub)).unwrap();

    prepare(&database.layout, &database.password).unwrap();

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
fn log_007_a_refused_superuser_gives_an_error_without_the_password() {
    let mut database = Database::new();
    let _server = database.start();
    database.password = crate::Password::from_bytes(&[5; 24]);

    let error = prepare(&database.layout, &database.password).unwrap_err();

    let text = format!("{error} {error:?}");
    assert!(text.contains("28P01"), "{text}");
    assert!(!text.contains(database.password.expose()));
}
