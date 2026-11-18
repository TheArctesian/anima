//! The file store and the extraction proposals (K-E4 Sections 9 and 10; FIL-001 to FIL-005,
//! VIS-001 to VIS-004). A file is one piece, sealed with the data key of the extension that owns
//! its record and kept by content hash; the database keeps what is known of it. An extraction is
//! only a proposal until the operator confirms it.

pub mod graphql;
mod location;
mod model;
mod parts;
mod pieces;
mod proposals;
mod removal;
mod routes;
mod store;
mod stream;
mod trail;
mod upload;

use std::{io, path::Path, sync::Arc};

use async_trait::async_trait;
use sqlx::{PgConnection, PgPool};

pub use location::{CLEANED_TYPES, Position};
pub use model::{Archive, Attachment, MAX_FILE, Removed};
pub use parts::{PlainStream, SealedStream};
pub use pieces::{FolderPieces, Pieces, StoreError};
pub use proposals::{
    Field, FieldType, NewProposal, Origin, Proposal, ProposalState, confirm, confirmed, propose,
    reject,
};
pub use removal::{delete_archive, remove_attachment};
pub use routes::router;
pub use stream::StreamError;

pub(crate) use crate::engine::data_keys::hex;
use crate::{
    engine::{
        data_keys::{DataKeyError, DataKeys},
        records::Reference,
    },
    ids::ArchiveId,
};

/// The most attachments [`attachments`] gives for one record.
pub const MAX_LIST: i64 = 200;

/// The domain that derives records from an archive could not delete them now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the records cannot be deleted now")]
pub struct Unavailable;

/// A domain that makes records from an archive, given to the files domain at wiring, so that
/// deleting the archive deletes its records too (FIL-004).
#[async_trait]
pub trait Derived: Send + Sync {
    /// The domain, for the audit entry of the delete.
    fn name(&self) -> &'static str;
    /// Deletes each record that came from `archive`, in the caller's transaction, and gives
    /// their count.
    async fn delete_from_archive(
        &self,
        conn: &mut PgConnection,
        archive: ArchiveId,
    ) -> Result<u64, Unavailable>;
}

/// Why a files operation failed. Messages hold no byte of a file, no key and no plain value
/// (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("the audit entry cannot be written ({0})")]
    Trail(#[from] crate::fleet::events::Error),
    /// No `ANIMA_FILES_DIR` or no `ANIMA_DATA_KEY_DIR`.
    #[error("the hub has no file store")]
    Unavailable,
    #[error("the data key cannot be used: {0}")]
    Key(#[from] DataKeyError),
    #[error("the file store failed: {0}")]
    Store(#[from] StoreError),
    #[error("the staging copy failed ({0})")]
    Staging(io::ErrorKind),
    /// A stored piece that does not open with its key and hash.
    #[error("the stored piece does not open")]
    Damaged,
    #[error("the file is larger than 256 MB")]
    TooLarge,
    /// The body of the upload ended with an error of its sender.
    #[error("the upload did not arrive whole")]
    Body,
    #[error("no such {0}")]
    NotFound(&'static str),
    /// A proposal that the operator already decided.
    #[error("the proposal is decided and does not change")]
    Decided,
    #[error("{0} records cannot be deleted now")]
    Derived(&'static str),
    /// Input the operator can fix: the field and what to give.
    #[error("{} is not valid", .0.iter().map(|bad| bad.field.as_str()).collect::<Vec<_>>().join(", "))]
    Invalid(Vec<Bad>),
}

/// One input field the operator must fix, with what to give.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bad {
    pub field: String,
    pub message: &'static str,
}

impl Bad {
    #[must_use]
    pub fn new(field: impl Into<String>, message: &'static str) -> Self {
        Self {
            field: field.into(),
            message,
        }
    }
}

/// The file store of the hub: the pieces, the data keys that seal them and the domains that
/// derive records from archives. Without pieces or keys every upload is `Unavailable`.
#[derive(Clone)]
pub struct Store {
    pieces: Option<Arc<dyn Pieces>>,
    staging: Option<std::path::PathBuf>,
    keys: DataKeys,
    derived: Arc<[Arc<dyn Derived>]>,
}

impl Store {
    /// No file store: `ANIMA_FILES_DIR` is not set.
    #[must_use]
    pub fn off() -> Self {
        Self {
            pieces: None,
            staging: None,
            keys: DataKeys::none(),
            derived: Arc::new([]),
        }
    }

    /// The pieces in `folder` (`ANIMA_FILES_DIR`), sealed with `keys`; staging copies go to
    /// `<folder>/.staging`, which no key name can be.
    #[must_use]
    pub fn in_folder(folder: Option<&Path>, keys: DataKeys) -> Self {
        let Some(folder) = folder else {
            return Self {
                keys,
                ..Self::off()
            };
        };
        Self {
            pieces: Some(Arc::new(FolderPieces::new(folder.to_path_buf()))),
            staging: Some(folder.join(".staging")),
            keys,
            derived: Arc::new([]),
        }
    }

    /// The same store, with the domains that derive records from archives (FIL-004).
    #[must_use]
    pub fn deriving(self, derived: Vec<Arc<dyn Derived>>) -> Self {
        Self {
            derived: derived.into(),
            ..self
        }
    }

    /// Whether uploads can work: the hub has pieces and data keys.
    #[must_use]
    pub fn available(&self) -> bool {
        self.pieces.is_some() && self.keys.available()
    }

    fn pieces(&self) -> Result<&Arc<dyn Pieces>, Error> {
        self.pieces.as_ref().ok_or(Error::Unavailable)
    }
}

/// The attachments of `owner`, newest first, at most [`MAX_LIST`].
///
/// # Errors
/// `Database` when the read fails.
pub async fn attachments(db: &PgPool, owner: &Reference) -> Result<Vec<Attachment>, Error> {
    Ok(store::attachments(db, owner, MAX_LIST).await?)
}

/// Makes an archive for `owner` in the caller's transaction.
///
/// # Errors
/// `Invalid` for a name that is not one line of 1 to 200 characters; `Database`.
pub async fn create_archive(
    conn: &mut PgConnection,
    (owner, name): (Reference, &str),
    (at, by): (chrono::DateTime<chrono::Utc>, &str),
) -> Result<Archive, Error> {
    let name = model::name(name)
        .ok_or_else(|| Error::Invalid(vec![Bad::new("name", model::NAME_RULE)]))?;
    let archive = Archive {
        id: uuid::Uuid::now_v7().into(),
        owner,
        name,
        created_at: at,
    };
    store::insert_archive(conn, &archive, by).await?;
    Ok(archive)
}

#[cfg(test)]
mod tests;
