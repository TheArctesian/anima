//! Deleting attachments and archives (FIL-004): the rows in the caller's transaction, then each
//! piece that no attachment uses, while its lock is held.

use std::collections::BTreeSet;

use sqlx::PgConnection;

use super::{Error, Store, model::Archive, model::Removed, store};
use crate::ids::{ArchiveId, AttachmentId};

/// Deletes one attachment in the caller's transaction, and its piece when no other attachment
/// has the same hash and key. `None` when there is no such attachment.
///
/// # Errors
/// `Unavailable` with no file store; `Database`, `Store`. The caller rolls back.
pub async fn remove_attachment(
    conn: &mut PgConnection,
    store: &Store,
    id: AttachmentId,
) -> Result<Option<Removed>, Error> {
    store.pieces()?;
    let Some(attachment) = store::delete_attachment(conn, id).await? else {
        return Ok(None);
    };
    let mut removed = Removed {
        attachments: vec![attachment],
        ..Removed::default()
    };
    removed.pieces = remove_unused(conn, store, &removed).await?;
    Ok(Some(removed))
}

/// Deletes an archive in the caller's transaction (FIL-004): first each record that a
/// `Derived` domain made from it, then each attachment of it, then the archive, then each piece
/// that no attachment uses now. `None` when there is no such archive.
///
/// # Errors
/// `Unavailable` with no file store; `Derived` when a domain cannot delete its records;
/// `Database`, `Store`. The caller rolls back.
pub async fn delete_archive(
    conn: &mut PgConnection,
    store: &Store,
    id: ArchiveId,
) -> Result<Option<(Archive, Removed)>, Error> {
    store.pieces()?;
    if store::archive(&mut *conn, id).await?.is_none() {
        return Ok(None);
    }
    let mut removed = Removed::default();
    for derived in store.derived.iter() {
        let count = derived
            .delete_from_archive(conn, id)
            .await
            .map_err(|_| Error::Derived(derived.name()))?;
        removed.derived.push((derived.name(), count));
    }
    removed.attachments = store::delete_archive_attachments(conn, id).await?;
    let archive = store::delete_archive(conn, id)
        .await?
        .ok_or(Error::NotFound("archive"))?;
    removed.pieces = remove_unused(conn, store, &removed).await?;
    Ok(Some((archive, removed)))
}

/// Removes the rows and the pieces of the attachments' pieces that no attachment uses now.
/// The locks are taken in one order, so two deletes cannot wait for each other.
async fn remove_unused(
    conn: &mut PgConnection,
    store: &Store,
    removed: &Removed,
) -> Result<Vec<(String, String)>, Error> {
    let keys: BTreeSet<String> = removed
        .attachments
        .iter()
        .map(|attachment| store::piece_key(&attachment.key_name, &attachment.hash))
        .collect();
    let keys: Vec<String> = keys.into_iter().collect();
    for key in &keys {
        store::lock_piece(conn, key).await?;
    }
    let unused = store::delete_unused_pieces(conn, &keys).await?;
    let pieces = store.pieces()?;
    for (key_name, hash) in &unused {
        pieces.remove(key_name, hash).await?;
    }
    Ok(unused)
}
