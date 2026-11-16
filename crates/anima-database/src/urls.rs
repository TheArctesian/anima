//! The URL files: one file for each login, holding the connection URL that the hub reads
//! through `ANIMA_DATABASE_URL_FILE` (DB-002). Each URL names the Unix socket, so the hub
//! connects as it does to any PostgreSQL and needs no setting of its own for this database.
//!
//! A URL holds a password. The files have mode 0600 in a folder with mode 0700, and nothing
//! here prints or logs one (LOG-007).

use std::fs;
use std::io::{self, Write};

use zeroize::Zeroizing;

use crate::{Layout, Password, Role, owner_only};

#[cfg(test)]
mod tests;

/// The database of the hub (K2).
pub const DATABASE: &str = "anima";

#[derive(Debug, thiserror::Error)]
pub enum UrlError {
    #[error("could not {step} a database URL file")]
    Io {
        step: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("a database URL file does not hold a URL this program wrote")]
    Form,
}

impl Role {
    /// The login's name in PostgreSQL.
    #[must_use]
    pub const fn login(self) -> &'static str {
        match self {
            Self::Owner => "anima_owner",
            Self::Hub => "anima_hub",
        }
    }

    /// The database the login's URL opens.
    #[must_use]
    pub const fn database(self) -> &'static str {
        match self {
            Self::Owner | Self::Hub => DATABASE,
        }
    }
}

/// Writes `role`'s URL file. The file appears whole or not at all: it is written beside its
/// place and moved in, so a crash never leaves half a URL for the hub to read.
///
/// # Errors
/// When the file cannot be written or moved.
pub fn write(layout: &Layout, role: Role, password: &Password) -> Result<(), UrlError> {
    let file = layout.url_file(role);
    let mut beside = file.clone().into_os_string();
    beside.push(".new");
    let text = url(layout, role, password);
    owner_only::folder(&layout.urls_folder()).map_err(|source| UrlError::Io {
        step: "make the folder of",
        source,
    })?;
    owner_only::new_file(beside.as_ref())
        .and_then(|mut new| {
            new.write_all(text.as_bytes())?;
            new.sync_all()
        })
        .map_err(|source| UrlError::Io {
            step: "write",
            source,
        })?;
    fs::rename(&beside, &file).map_err(|source| UrlError::Io {
        step: "move in",
        source,
    })
}

/// The password in `role`'s URL file, or `None` when the file is not there yet.
///
/// # Errors
/// When the file cannot be read or is not a URL of [`write`].
pub fn password(layout: &Layout, role: Role) -> Result<Option<Password>, UrlError> {
    let text = match fs::read_to_string(layout.url_file(role)) {
        Ok(text) => Zeroizing::new(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(UrlError::Io {
                step: "read",
                source,
            });
        }
    };
    let password = text
        .strip_prefix("postgres://")
        .and_then(|rest| rest.strip_prefix(role.login()))
        .and_then(|rest| rest.strip_prefix(':'))
        .and_then(|rest| rest.split_once('@'))
        .map(|(password, _)| password)
        .filter(|password| {
            !password.is_empty() && password.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .ok_or(UrlError::Form)?;
    Ok(Some(Password::from_text(password.to_owned())))
}

/// `postgres://<login>:<password>@localhost:5432/<database>?host=<socket folder>`: the form
/// `sqlx` and libpq read as "connect through the Unix socket in that folder".
fn url(layout: &Layout, role: Role, password: &Password) -> Zeroizing<String> {
    let socket_folder = layout.socket_folder().as_os_str().as_encoded_bytes();
    Zeroizing::new(format!(
        "postgres://{}:{}@localhost:5432/{}?host={}",
        role.login(),
        password.expose(),
        role.database(),
        percent_encoded(socket_folder),
    ))
}

/// Every byte that is not an unreserved URL character becomes `%XX`.
fn percent_encoded(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            text.push(char::from(byte));
        } else {
            // Writing to a String cannot fail.
            let _ = write!(text, "%{byte:02X}");
        }
    }
    text
}
