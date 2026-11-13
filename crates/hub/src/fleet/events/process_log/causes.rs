//! What the process log may say about why a request failed: the hidden cause chain of a
//! GraphQL error, described without SQL text, values or tokens (H-5, API-010, LOG-007).

use std::{error::Error as StdError, sync::Arc};

use async_graphql::ServerError;

use crate::fleet::events::redaction::redact_text;

/// The type a domain stores as a GraphQL error's `source` so the API boundary can walk the
/// chain: `async_graphql::Error::new_with_source(Arc::new(cause) as Arc<dyn Error + Send + Sync>)`.
/// Domains write the std type (K3 gives them no `process_log` import). A source of any other
/// type cannot be read; only the error's own message is logged then.
pub(super) type Cause = Arc<dyn StdError + Send + Sync>;

/// The most links of a chain the log describes, so a cyclic or very long chain stays bounded.
const MAX_LINKS: usize = 8;

/// The error's cause chain as one line, links joined by `": "`.
///
/// - a hidden [`Cause`]: each link's text with secrets masked (`redact_text`), and a
///   `sqlx::Error` by its kind only, which ends the chain (its driver text may quote values);
/// - a `sqlx::Error` source (a resolver's `?`): its kind;
/// - otherwise the error's own message, masked.
pub(super) fn chain(error: &ServerError) -> String {
    if let Some(cause) = error.source::<Cause>() {
        return links(cause.as_ref());
    }
    if let Some(database) = error.source::<sqlx::Error>() {
        return database_kind(database);
    }
    redact_text(&error.message)
}

fn links(first: &(dyn StdError + 'static)) -> String {
    let mut described = Vec::new();
    let mut link = Some(first);
    while let Some(current) = link {
        if described.len() == MAX_LINKS {
            described.push("…".to_owned());
            break;
        }
        if let Some(database) = current.downcast_ref::<sqlx::Error>() {
            described.push(database_kind(database));
            break;
        }
        described.push(redact_text(&current.to_string()));
        link = current.source();
    }
    described.join(": ")
}

/// A database error by kind: the SQLSTATE and constraint name for a server error, a fixed
/// word otherwise. Never the server's message, which can quote a value.
pub(super) fn database_kind(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(database) => {
            let code = database.code().unwrap_or_default();
            match database.constraint() {
                Some(constraint) => format!("database error {code} on {constraint}"),
                None => format!("database error {code}"),
            }
        }
        sqlx::Error::Io(io) => format!("database connection: {:?}", io.kind()),
        other => database_word(other).to_owned(),
    }
}

fn database_word(error: &sqlx::Error) -> &'static str {
    match error {
        sqlx::Error::PoolTimedOut => "database pool timed out",
        sqlx::Error::PoolClosed => "database pool closed",
        sqlx::Error::Tls(_) => "database tls",
        sqlx::Error::Protocol(_) => "database protocol",
        sqlx::Error::RowNotFound => "database row not found",
        sqlx::Error::TypeNotFound { .. } => "database type not found",
        sqlx::Error::ColumnIndexOutOfBounds { .. } | sqlx::Error::ColumnNotFound(_) => {
            "database column not found"
        }
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => "database decode",
        sqlx::Error::Encode(_) => "database encode",
        sqlx::Error::Configuration(_) => "database configuration",
        sqlx::Error::WorkerCrashed => "database worker crashed",
        _ => "database",
    }
}
