//! What the removal of a device does to the operational keys (K-E5 Section 2.1, "After
//! removal"): its grants end, and the hub says which keys its server had, because a value that
//! a server fetched cannot be taken back from it.

use sqlx::PgConnection;

use super::{Error, store::consumers, trail};
use crate::{fleet::events::TrailEntry, ids::DeviceId};

/// Ends every grant of `device`, in the caller's transaction, before the device is deleted.
/// Gives one entry for the trail for each key that its daemon fetched or held: it names the
/// key and the version (no value), so the operator knows which keys to revoke at their
/// provider when the server is lost. The caller writes the entries in the same transaction.
///
/// # Errors
/// `Database`.
pub async fn forget_device(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<Vec<TrailEntry>, Error> {
    let had = consumers::delete_of_device(conn, device).await?;
    Ok(had
        .into_iter()
        .map(|(key, name, version)| trail::device_removed((key, &name), version, device))
        .collect())
}
