//! The names of the database and of the user of an application, and the quoting of an
//! identifier in a statement.

use std::fmt;

/// The longest identifier of PostgreSQL, in bytes.
const LONGEST: usize = 63;

/// Names that PostgreSQL or its image keeps for itself.
const KEPT: [&str; 4] = ["postgres", "template0", "template1", "public"];

/// Why a name was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("a database name is [a-z][a-z0-9_]{{0,62}} and not a name that PostgreSQL keeps")]
    NotPermitted,
}

/// `[a-z][a-z0-9_]{0,62}`, not a kept name and not with the prefix `pg_`.
fn is_database_name(text: &str) -> bool {
    text.bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && text.len() <= LONGEST
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !KEPT.contains(&text)
        && !text.starts_with("pg_")
}

/// The name of an extension: `[a-z][a-z0-9_-]{0,62}` (for example `pg_trgm`, `uuid-ossp`).
#[must_use]
pub fn is_extension(text: &str) -> bool {
    text.bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && text.len() <= LONGEST
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
}

/// The name of one database or one user, checked: it is safe in a quoted identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatabaseName(String);

impl DatabaseName {
    /// # Errors
    /// [`NameError::NotPermitted`] for a text that is not `[a-z][a-z0-9_]{0,62}`, is a name
    /// that PostgreSQL keeps, or starts with `pg_`.
    pub fn new(text: &str) -> Result<Self, NameError> {
        is_database_name(text)
            .then(|| Self(text.to_owned()))
            .ok_or(NameError::NotPermitted)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The name as a quoted identifier of a statement. The name has no `"`, so nothing is
    /// doubled; the quotes keep a key word of SQL a name.
    #[must_use]
    pub fn quoted(&self) -> String {
        format!("\"{}\"", self.0)
    }
}

impl fmt::Display for DatabaseName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The database and the user of one application. The user owns only that database.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    pub database: DatabaseName,
    pub user: DatabaseName,
}

impl Names {
    /// The names of the application `application`: each `-` of the identifier becomes `_`,
    /// and both names are that text. Two identifiers that differ only in `-` and `_` get the
    /// same names; the install refuses a database that exists.
    ///
    /// # Errors
    /// [`NameError::NotPermitted`] when the text is not a permitted name.
    pub fn for_application(application: &str) -> Result<Self, NameError> {
        let name = DatabaseName::new(&application.replace('-', "_"))?;
        Ok(Self {
            database: name.clone(),
            user: name,
        })
    }
}
