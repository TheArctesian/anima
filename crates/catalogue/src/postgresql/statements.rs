//! The fixed statements that make and delete the database and the user of an application,
//! and a database for the way back. The program on the machine runs them as the superuser of
//! the server, through the standard input of `psql`, so that the password is in no argument.

use std::fmt;

use super::names::{DatabaseName, Names};

/// The shortest and the longest password, in characters.
const PASSWORD_LENGTHS: std::ops::RangeInclusive<usize> = 16..=128;

/// Why a password was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PasswordError {
    #[error("a password is 16 to 128 printable characters, with no space, quote or backslash")]
    NotPermitted,
}

/// The password of the user of an application. The program on the machine makes it
/// (OD-HOST-42 A); this crate never makes one. It is printable ASCII with no space, `'`, `"`
/// or `\`, so that it is the same text in a quoted literal of SQL with each setting of the
/// server. No `Debug` or `Display` shows it.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    /// # Errors
    /// [`PasswordError::NotPermitted`] for a text of another length or with another character.
    pub fn new(text: &str) -> Result<Self, PasswordError> {
        let permitted = PASSWORD_LENGTHS.contains(&text.len())
            && text
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !b"'\"\\".contains(&byte));
        permitted
            .then(|| Self(text.to_owned()))
            .ok_or(PasswordError::NotPermitted)
    }

    pub(super) fn text(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Password(hidden)")
    }
}

/// One statement of SQL. A statement that holds a password does not show it in `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Statement {
    text: String,
    secret: bool,
}

impl Statement {
    fn plain(text: String) -> Self {
        Self {
            text,
            secret: false,
        }
    }

    /// The text to give to the server.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.secret {
            formatter.write_str("Statement(holds a password)")
        } else {
            write!(formatter, "Statement({:?})", self.text)
        }
    }
}

/// The statements that make the user and the database of one application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    /// Run in the database `postgres`, in this order.
    pub server: Vec<Statement>,
    /// Run in the new database, as the superuser: each extension that the recipe needs.
    pub database: Vec<Statement>,
}

/// The user `names.user` with `password` and no other right, the database `names.database`
/// that it owns and that no other user may reach, and each extension of `extensions` in it.
/// Each extension must be a name of [`super::is_extension`]; the caller checked it with the
/// recipe (`database_not_described`).
#[must_use]
pub fn setup(names: &Names, password: &Password, extensions: &[String]) -> Setup {
    let user = names.user.quoted();
    let database = names.database.quoted();
    let role = Statement {
        text: format!(
            "CREATE ROLE {user} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT \
             NOREPLICATION NOBYPASSRLS PASSWORD '{}'",
            password.text()
        ),
        secret: true,
    };
    Setup {
        server: vec![
            role,
            new_database(&names.database, &names.user),
            Statement::plain(format!("REVOKE ALL ON DATABASE {database} FROM PUBLIC")),
            Statement::plain(format!(
                "GRANT CONNECT, TEMPORARY ON DATABASE {database} TO {user}"
            )),
        ],
        database: extensions
            .iter()
            .filter(|name| super::is_extension(name))
            .map(|name| Statement::plain(format!("CREATE EXTENSION IF NOT EXISTS \"{name}\"")))
            .collect(),
    }
}

/// A new, empty database `target` owned by `owner`, from the clean template.
#[must_use]
pub fn new_database(target: &DatabaseName, owner: &DatabaseName) -> Statement {
    Statement::plain(format!(
        "CREATE DATABASE {} OWNER {} ENCODING 'UTF8' TEMPLATE template0",
        target.quoted(),
        owner.quoted()
    ))
}

/// The deletion of the database `target`, also when a session is open on it.
#[must_use]
pub fn drop_database(target: &DatabaseName) -> Statement {
    Statement::plain(format!(
        "DROP DATABASE IF EXISTS {} WITH (FORCE)",
        target.quoted()
    ))
}

/// The deletion of the database and of the user of an application. The program runs it only
/// with the setting "The hub may delete volumes" at the machine (R-230).
#[must_use]
pub fn removal(names: &Names) -> Vec<Statement> {
    vec![
        drop_database(&names.database),
        Statement::plain(format!("DROP ROLE IF EXISTS {}", names.user.quoted())),
    ]
}

/// The name of the database of the way back number `number`: `<database>_back_<number>`.
///
/// # Errors
/// [`super::NameError::NotPermitted`] when the name is longer than 63 bytes.
pub fn way_back_database(names: &Names, number: u32) -> Result<DatabaseName, super::NameError> {
    DatabaseName::new(&format!("{}_back_{number}", names.database))
}
