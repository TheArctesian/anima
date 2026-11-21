//! The copy of the database of an application (`pg_dump` in the custom format), the way back
//! (`pg_restore` into a new database) and the test of the way back (Section 14.3), as
//! argument lists that the program runs in the server container. HOST-042 is not approved:
//! nothing runs them yet.

use std::collections::BTreeSet;

use super::{
    names::{DatabaseName, NameError, Names},
    statements::{Statement, drop_database, new_database},
};

/// The superuser of the official image.
const SUPERUSER: &str = "--username=postgres";

/// The tables of a database, as `schema.table`, one on each line, with no header.
const TABLES: &str = "SELECT schemaname || '.' || tablename FROM pg_catalog.pg_tables \
     WHERE schemaname NOT IN ('pg_catalog', 'information_schema') ORDER BY 1";

/// The file of one copy in the server container: an absolute path of letters, digits, `/`,
/// `.`, `_` and `-`, with no `..` part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpFile(String);

impl DumpFile {
    /// `None` for a path that is not permitted.
    #[must_use]
    pub fn new(path: &str) -> Option<Self> {
        let permitted = path.starts_with('/')
            && path.len() <= 4096
            && path
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
            && !path.split('/').any(|part| part == "..");
        permitted.then(|| Self(path.to_owned()))
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.0
    }
}

/// The copy of the database of `names` into `copy`, in the custom format of `pg_dump`.
#[must_use]
pub fn dump_command(names: &Names, copy: &DumpFile) -> Vec<String> {
    vec![
        "pg_dump".to_owned(),
        "--format=custom".to_owned(),
        "--no-password".to_owned(),
        SUPERUSER.to_owned(),
        format!("--dbname={}", names.database),
        format!("--file={}", copy.path()),
    ]
}

/// The restore of `copy` into the empty database `target`. Each object belongs to `owner`;
/// the restore stops at the first error and changes nothing then.
#[must_use]
pub fn restore_command(
    target: &DatabaseName,
    owner: &DatabaseName,
    copy: &DumpFile,
) -> Vec<String> {
    vec![
        "pg_restore".to_owned(),
        "--no-password".to_owned(),
        SUPERUSER.to_owned(),
        format!("--dbname={target}"),
        "--no-owner".to_owned(),
        format!("--role={owner}"),
        "--exit-on-error".to_owned(),
        "--single-transaction".to_owned(),
        copy.path().to_owned(),
    ]
}

/// The table of contents of `copy`, which [`tables_in_list`] reads.
#[must_use]
pub fn list_command(copy: &DumpFile) -> Vec<String> {
    vec![
        "pg_restore".to_owned(),
        "--list".to_owned(),
        copy.path().to_owned(),
    ]
}

/// The query of the tables of a database, which [`check_command`] runs.
#[must_use]
pub const fn tables_query() -> &'static str {
    TABLES
}

/// The tables of the database `target`, one `schema.table` on each line.
#[must_use]
pub fn check_command(target: &DatabaseName) -> Vec<String> {
    vec![
        "psql".to_owned(),
        "--no-psqlrc".to_owned(),
        "--no-password".to_owned(),
        SUPERUSER.to_owned(),
        "--tuples-only".to_owned(),
        "--no-align".to_owned(),
        format!("--dbname={target}"),
        format!("--command={TABLES}"),
    ]
}

/// Each table of the table of contents that `pg_restore --list` wrote, as `schema.table`. A
/// line of a table is `<id>; <oid> <oid> TABLE <schema> <table> <owner>`; a line of its rows
/// (`TABLE DATA`) and a comment (`;`) are not tables.
#[must_use]
pub fn tables_in_list(list: &str) -> BTreeSet<String> {
    list.lines()
        .filter(|line| !line.trim_start().starts_with(';'))
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                [_, _, _, "TABLE", schema, table, ..] if *schema != "DATA" => {
                    Some(format!("{schema}.{table}"))
                }
                _ => None,
            }
        })
        .collect()
}

/// Each table of `listed` that is not a line of `found`, the answer of [`check_command`]. An
/// empty list: the way back is good.
#[must_use]
pub fn missing_tables(listed: &BTreeSet<String>, found: &str) -> Vec<String> {
    let found: BTreeSet<&str> = found.lines().map(str::trim).collect();
    listed
        .iter()
        .filter(|table| !found.contains(table.as_str()))
        .cloned()
        .collect()
}

/// The steps of the way back or of its test, in order: make `target`, list the copy, restore
/// it, read the tables, and (for the test only) delete `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restore {
    pub target: DatabaseName,
    pub create: Statement,
    pub list: Vec<String>,
    pub restore: Vec<String>,
    pub tables: Vec<String>,
    /// For the test of the way back only. The way back keeps its database: the operator
    /// switches the application to it, and the old database stays until he deletes it.
    pub drop: Option<Statement>,
}

fn restore(names: &Names, copy: &DumpFile, target: DatabaseName, test: bool) -> Restore {
    Restore {
        create: new_database(&target, &names.user),
        list: list_command(copy),
        restore: restore_command(&target, &names.user, copy),
        tables: check_command(&target),
        drop: test.then(|| drop_database(&target)),
        target,
    }
}

/// The way back number `number`: `copy` restored into the new database
/// `<database>__back_<number>`.
///
/// # Errors
/// [`NameError::NotPermitted`] when that name is longer than 63 bytes.
pub fn way_back(names: &Names, copy: &DumpFile, number: u32) -> Result<Restore, NameError> {
    let target = super::statements::way_back_database(names, number)?;
    Ok(restore(names, copy, target, false))
}

/// The test of the way back: `copy` restored into `<database>__check`, each table of the copy
/// found there ([`missing_tables`]), and the database deleted.
///
/// # Errors
/// [`NameError::NotPermitted`] when that name is longer than 63 bytes.
pub fn way_back_test(names: &Names, copy: &DumpFile) -> Result<Restore, NameError> {
    let target = DatabaseName::new(&format!("{}__check", names.database))?;
    Ok(restore(names, copy, target, true))
}
