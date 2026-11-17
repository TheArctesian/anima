//! Record links: how a record of one extension points at a record of another, so that no
//! extension imports another or waits for it (RLK-001, RLK-002, RLK-004). One table holds
//! every link; a link to a record that does not exist yet keeps a text label.

pub mod graphql;
mod model;
mod store;
mod trail;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

pub use model::{Link, NewLink, Outcome, Relation, label};

use crate::{
    engine::records::{Reference, Subject},
    ids::RecordLinkId,
};

/// The most links [`touching`] returns for one record.
pub const MAX_LINKS: usize = 200;

/// Why a links operation failed. Messages hold no SQL and no input value.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// The source and the target are the same record.
    #[error("a record cannot link to itself")]
    SelfLink,
    /// A side has a subject kind and is not the text of that subject (RLK-004).
    #[error("{0} is not a valid subject key")]
    BadSubject(&'static str),
}

/// `side` when it may be one side of a link: any reference, except text that has a subject
/// kind and is not the one spelling of that subject.
fn checked(reference: &Reference, side: &'static str) -> Result<(), Error> {
    if Subject::owns(reference) && Subject::of(reference).is_none() {
        Err(Error::BadSubject(side))
    } else {
        Ok(())
    }
}

/// Makes the link in the caller's transaction, or finds the one that has the same source,
/// target and kind: a second call with the same link writes nothing, and one with a new label
/// writes the label. Neither record has to exist (RLK-002).
///
/// # Errors
/// `SelfLink`, `BadSubject`; `Database` when a statement fails. The caller rolls back.
pub async fn connect(
    conn: &mut PgConnection,
    new: NewLink,
    (at, by): (DateTime<Utc>, &str),
) -> Result<(Link, Outcome), Error> {
    if new.source == new.target {
        return Err(Error::SelfLink);
    }
    checked(&new.source, "source")?;
    checked(&new.target, "target")?;
    if let Some(made) = store::insert(conn, &new, (at, by)).await? {
        return Ok((made, Outcome::Made));
    }
    let Some(stored) = store::existing(conn, &new).await? else {
        // Deleted between the two statements: make it now.
        let made = store::insert(conn, &new, (at, by)).await?;
        return made
            .map(|link| (link, Outcome::Made))
            .ok_or(Error::Database(sqlx::Error::RowNotFound));
    };
    match new.label {
        Some(label) if stored.label.as_deref() != Some(&label) => {
            store::set_label(conn, stored.id, &label).await?;
            let relabelled = Link {
                label: Some(label),
                ..stored
            };
            Ok((relabelled, Outcome::Relabelled))
        }
        _ => Ok((stored, Outcome::Kept)),
    }
}

/// Removes one link in the caller's transaction and returns it; `None` when there is no such
/// link. The two records are not touched.
///
/// # Errors
/// `Database` when the statement fails.
pub async fn disconnect(conn: &mut PgConnection, id: RecordLinkId) -> Result<Option<Link>, Error> {
    Ok(store::delete(conn, id).await?)
}

/// The links that start or end at `record`, newest first, at most [`MAX_LINKS`].
///
/// # Errors
/// `Database` when the read fails.
pub async fn touching(db: &PgPool, record: &Reference) -> Result<Vec<Link>, Error> {
    let limit = i64::try_from(MAX_LINKS).unwrap_or(i64::MAX);
    Ok(store::touching(db, record, limit).await?)
}

/// What to call `record` when it names itself: a date or a repository is its own title. Any
/// other record gets its title from the domain that owns it; until that domain exists a link
/// shows its label (RLK-002).
#[must_use]
pub fn own_title(record: &Reference) -> Option<String> {
    match Subject::of(record)? {
        Subject::Repository(name) => Some(name),
        Subject::Date(date) => Some(date.to_string()),
        Subject::Place(_) | Subject::Person(_) => None,
    }
}
