//! The names, the statements, the connection values and the commands of the PostgreSQL entry.

use super::{
    Address, DatabaseName, DumpFile, NameError, Names, Password, PasswordError, dump_command,
    missing_tables, removal, setup, tables_in_list, values, way_back, way_back_test,
};
use crate::entry::Variables;

const PASSWORD: &str = "s3cretPassw0rdOf24chars9";

fn notes() -> Names {
    Names::for_application("notes").expect("a permitted name")
}

fn password() -> Password {
    Password::new(PASSWORD).expect("a permitted password")
}

fn texts(statements: &[super::Statement]) -> Vec<&str> {
    statements.iter().map(super::Statement::text).collect()
}

#[test]
fn h24_the_names_come_from_the_application_identifier() {
    let names = Names::for_application("owntracks-recorder").expect("a permitted name");
    assert_eq!(names.database.as_str(), "owntracks_recorder");
    assert_eq!(names.user.as_str(), "owntracks_recorder");
    let longest = "a".repeat(63);
    assert!(Names::for_application(&longest).is_ok());
}

#[test]
fn sf_revcatpg_two_identifiers_never_get_the_same_database_or_user() {
    let hyphen = Names::for_application("my-app").expect("a permitted name");
    assert_eq!(hyphen.database.as_str(), "my_app");
    assert_eq!(
        Names::for_application("my_app"),
        Err(NameError::NotPermitted)
    );
    let dump = DumpFile::new("/copies/notes.dump").expect("a permitted path");
    let back = way_back(&notes(), &dump, 1).expect("a permitted name");
    assert_eq!(
        Names::for_application("notes-back-1").map(|names| names.database),
        Ok(DatabaseName::new("notes_back_1").expect("a name"))
    );
    assert_ne!(back.target.as_str(), "notes_back_1");
}

#[test]
fn h24_a_name_that_is_not_permitted_is_refused() {
    let too_long = "a".repeat(64);
    for refused in [
        "",
        "1notes",
        "_notes",
        "Notes",
        "no tes",
        "no\"tes",
        "no;tes",
        "postgres",
        "template1",
        "public",
        "pg_notes",
        "my_app",
        "my--app",
        &too_long,
    ] {
        assert_eq!(
            Names::for_application(refused),
            Err(NameError::NotPermitted),
            "{refused}"
        );
    }
}

#[test]
fn h24_the_setup_makes_a_user_that_owns_only_its_database() {
    let made = setup(
        &notes(),
        &password(),
        &["pg_trgm".to_owned(), "uuid-ossp".to_owned()],
    );
    assert_eq!(
        texts(&made.server),
        [
            "CREATE ROLE \"notes\" LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT \
             NOREPLICATION NOBYPASSRLS PASSWORD 's3cretPassw0rdOf24chars9'",
            "CREATE DATABASE \"notes\" OWNER \"notes\" ENCODING 'UTF8' TEMPLATE template0",
            "REVOKE ALL ON DATABASE \"notes\" FROM PUBLIC",
            "GRANT CONNECT, TEMPORARY ON DATABASE \"notes\" TO \"notes\"",
        ]
    );
    assert_eq!(
        texts(&made.database),
        [
            "CREATE EXTENSION IF NOT EXISTS \"pg_trgm\"",
            "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\""
        ]
    );
    let shown = format!("{made:?}");
    assert!(!shown.contains(PASSWORD), "{shown}");
}

#[test]
fn sf_revcatpg_an_extension_that_is_not_in_the_list_makes_no_statement() {
    let made = setup(
        &notes(),
        &password(),
        &[
            "x\"; DROP DATABASE notes; --".to_owned(),
            "plpython3u".to_owned(),
            "dblink".to_owned(),
            "adminpack".to_owned(),
        ],
    );
    assert_eq!(made.database, []);
    let listed = setup(&notes(), &password(), &["vector".to_owned()]);
    assert_eq!(
        texts(&listed.database),
        ["CREATE EXTENSION IF NOT EXISTS \"vector\""]
    );
}

#[test]
fn sf_revcatpg_a_password_that_is_not_letters_and_digits_or_has_a_wrong_length_is_refused() {
    for refused in [
        "short",
        "abcdefghijklmnopqrstu",
        "has$dollar0in0it0of0chars",
        "has`backtick0in0it0chars",
        "has#hash0in0it0of0chars0",
        "has=equals0in0it0of0chars",
        "has-hyphen0in0it0of0chars",
        "has'quote-in-it-16chars",
        "has\"quote-in-it-16chars",
        "has\\backslash-16chars",
        "has a space in it 16chars",
        "non-ascii-é-password-16",
        &"a".repeat(129),
    ] {
        assert_eq!(
            Password::new(refused),
            Err(PasswordError::NotPermitted),
            "{refused}"
        );
    }
    assert!(Password::new(&"a".repeat(22)).is_ok());
    assert!(!format!("{:?}", password()).contains(PASSWORD));
}

#[test]
fn h24_the_removal_deletes_the_database_then_the_user() {
    assert_eq!(
        texts(&removal(&notes())),
        [
            "DROP DATABASE IF EXISTS \"notes\" WITH (FORCE)",
            "DROP ROLE IF EXISTS \"notes\""
        ]
    );
}

#[test]
fn h24_the_connection_values_go_into_the_variables_that_the_recipe_names() {
    let variables = Variables {
        url: Some("DATABASE_URL".to_owned()),
        host: Some("DB_HOST".to_owned()),
        name: Some("DB_NAME".to_owned()),
        password: Some("DB_PASSWORD".to_owned()),
        ..Variables::default()
    };
    let address = Address {
        host: "postgresql".to_owned(),
        port: 5432,
    };
    let written = values(&variables, &address, &notes(), &password()).expect("a host name");
    let pairs: Vec<(&str, &str)> = written
        .iter()
        .map(|value| (value.variable.as_str(), value.value.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            (
                "DATABASE_URL",
                "postgresql://notes:s3cretPassw0rdOf24chars9@postgresql:5432/notes"
            ),
            ("DB_HOST", "postgresql"),
            ("DB_NAME", "notes"),
            ("DB_PASSWORD", PASSWORD),
        ]
    );
    let shown = format!("{written:?}");
    assert!(!shown.contains("s3cret"), "{shown}");
    let bad_host = Address {
        host: "db/../x".to_owned(),
        port: 5432,
    };
    assert_eq!(values(&variables, &bad_host, &notes(), &password()), None);
}

#[test]
fn h24_the_copy_is_pg_dump_in_the_custom_format() {
    let dump = DumpFile::new("/var/lib/postgresql/copies/notes.dump").expect("a permitted path");
    assert_eq!(
        dump_command(&notes(), &dump),
        [
            "pg_dump",
            "--format=custom",
            "--no-password",
            "--username=postgres",
            "--dbname=notes",
            "--file=/var/lib/postgresql/copies/notes.dump"
        ]
    );
    for refused in [
        "copies/notes.dump",
        "/var/../etc/passwd",
        "/tmp/a b",
        "/tmp/$(x)",
    ] {
        assert_eq!(DumpFile::new(refused), None, "{refused}");
    }
}

#[test]
fn h24_the_way_back_restores_into_a_new_database_and_keeps_it() {
    let dump = DumpFile::new("/copies/notes.dump").expect("a permitted path");
    let back = way_back(&notes(), &dump, 3).expect("a permitted name");
    assert_eq!(back.target.as_str(), "notes__back_3");
    assert_eq!(
        back.create.text(),
        "CREATE DATABASE \"notes__back_3\" OWNER \"notes\" ENCODING 'UTF8' TEMPLATE template0"
    );
    assert_eq!(
        back.restore,
        [
            "pg_restore",
            "--no-password",
            "--username=postgres",
            "--dbname=notes__back_3",
            "--no-owner",
            "--role=notes",
            "--exit-on-error",
            "--single-transaction",
            "/copies/notes.dump"
        ]
    );
    assert_eq!(back.drop, None);
    let long = Names::for_application(&"a".repeat(60)).expect("a permitted name");
    assert_eq!(way_back(&long, &dump, 1), Err(NameError::NotPermitted));
}

#[test]
fn h24_the_test_of_the_way_back_finds_each_table_of_the_copy_then_deletes_its_database() {
    let dump = DumpFile::new("/copies/notes.dump").expect("a permitted path");
    let test = way_back_test(&notes(), &dump).expect("a permitted name");
    assert_eq!(
        test.target,
        DatabaseName::new("notes__check").expect("a name")
    );
    assert_eq!(test.list, ["pg_restore", "--list", "/copies/notes.dump"]);
    assert!(test.tables.contains(&"--dbname=notes__check".to_owned()));
    assert_eq!(
        test.drop.as_ref().map(super::Statement::text),
        Some("DROP DATABASE IF EXISTS \"notes__check\" WITH (FORCE)")
    );
    let list = ";\n; Archive created at 2026-11-20 10:00:00 UTC\n;\n\
        215; 1259 16386 TABLE public notes notes\n\
        216; 1259 16390 TABLE public tags notes\n\
        3301; 0 16386 TABLE DATA public notes notes\n\
        3150; 2606 16394 CONSTRAINT public notes notes_pkey notes\n";
    let listed = tables_in_list(list);
    assert_eq!(
        listed.iter().map(String::as_str).collect::<Vec<_>>(),
        ["public.notes", "public.tags"]
    );
    assert_eq!(
        missing_tables(&listed, "public.notes\npublic.tags\n"),
        Vec::<String>::new()
    );
    assert_eq!(missing_tables(&listed, "public.notes\n"), ["public.tags"]);
}
