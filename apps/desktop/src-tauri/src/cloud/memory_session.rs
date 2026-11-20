//! The `WorkOS` refresh token of a phone development build, in memory (OD-M7 of the mobile plan): one
//! for each client ID, lost when the app stops, so the person signs in again. A build that leaves the
//! operator's computers must keep it in the key store of the operating system instead (MOB-006,
//! HMD-034; mobile plan M5-1). It never reaches the web view, a file or a log (LOG-007).

use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard, PoisonError},
};

use zeroize::Zeroizing;

use super::CloudError;

static KEPT: Mutex<BTreeMap<String, Zeroizing<String>>> = Mutex::new(BTreeMap::new());

fn kept() -> MutexGuard<'static, BTreeMap<String, Zeroizing<String>>> {
    KEPT.lock().unwrap_or_else(PoisonError::into_inner)
}

// The three functions answer `Result` as the key store of a computer does (`session.rs`), so the
// sign-in is the same code on a computer and on a phone.

#[expect(
    clippy::unnecessary_wraps,
    reason = "the key store of a computer can refuse; memory cannot"
)]
pub(super) fn read(client_id: &str) -> Result<Option<Zeroizing<String>>, CloudError> {
    Ok(kept().get(client_id).cloned())
}

/// Keeps `token` in place of the kept one: `WorkOS` may rotate it at each exchange.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the key store of a computer can refuse; memory cannot"
)]
pub(super) fn write(client_id: &str, token: &str) -> Result<(), CloudError> {
    kept().insert(client_id.to_owned(), Zeroizing::new(token.to_owned()));
    Ok(())
}

/// Forgets the kept token, as when `WorkOS` refuses it (HMD-034).
#[expect(
    clippy::unnecessary_wraps,
    reason = "the key store of a computer can refuse; memory cannot"
)]
pub(super) fn clear(client_id: &str) -> Result<(), CloudError> {
    kept().remove(client_id);
    Ok(())
}
