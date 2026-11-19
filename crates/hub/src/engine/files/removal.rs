//! Deleting attachments and archives (FIL-004): the rows in the caller's transaction, then,
//! after the caller committed, the stored bytes of each piece that has no row ([`discard`]).

use std::collections::BTreeSet;

use sqlx::{PgConnection, PgPool};

use super::{Error, Store, model::Archive, model::Removed, store};
use crate::ids::{ArchiveId, AttachmentId};

/// Deletes one attachment in the caller's transaction, and the row of its piece when no other
/// attachment has the same hash and key. `None` when there is no such attachment. The caller
/// commits, then gives `Removed::pieces` to [`discard`].
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
    removed.pieces = remove_unused(conn, &removed).await?;
    Ok(Some(removed))
}

/// Deletes an archive in the caller's transaction (FIL-004): first each record that a
/// `Derived` domain made from it, then each attachment of it, then the archive, then the row of each
/// piece that no attachment uses now. `None` when there is no such archive. The caller commits,
/// then gives `Removed::pieces` to [`discard`].
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
    removed.pieces = remove_unused(conn, &removed).await?;
    Ok(Some((archive, removed)))
}

/// Removes the row of each of the attachments' pieces that no attachment uses now. The locks
/// are taken in one order, so two deletes cannot wait for each other.
async fn remove_unused(
    conn: &mut PgConnection,
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
    Ok(store::delete_unused_pieces(conn, &keys).await?)
}

/// Removes the stored bytes of each of `pieces` (key name and hash) that has no row now. Call
/// it after the delete committed: a delete that rolls back then loses no bytes, and an upload
/// of the same bytes in the meantime keeps its piece, because each check holds the piece lock.
///
/// # Errors
/// `Unavailable` with no file store; `Database`, `Store`. The rows are deleted in each case;
/// a piece that stays is sealed bytes that no attachment names.
pub async fn discard(db: &PgPool, store: &Store, pieces: &[(String, String)]) -> Result<(), Error> {
    let folder = store.pieces()?;
    for (key_name, hash) in pieces {
        let key = store::piece_key(key_name, hash);
        let mut tx = db.begin().await?;
        store::lock_piece(&mut tx, &key).await?;
        if !store::piece_exists(&mut tx, &key).await? {
            folder.remove(key_name, hash).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}
