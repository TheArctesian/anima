//! The key cache at work beside the pusher: it says what it holds, follows the hub's current
//! versions, fetches what changed, and deletes itself when the rules say so (K-E5 Section 2.1).
//! The decisions are in [`cache`](super::cache); this is the I/O edge: the store and the hub.

use chrono::{DateTime, Utc};
use daemon_core::wire::{DaemonToken, KeyFetch, KeyVersion, KeysCurrent, RefusalCode};
use uuid::Uuid;

use super::cache::KeyCache;
use super::store::{KeyStore, KeyStoreError};
use crate::hub::HubClient;
use crate::hub::answer::{Answer, Cause};

/// The cache and where it is kept.
pub struct Keeper {
    cache: KeyCache,
    store: KeyStore,
}

/// What [`Keeper::follow`] needs of the pusher: its hub, its token and its device.
pub struct Asking<'a> {
    pub hub: &'a HubClient,
    pub token: &'a DaemonToken,
    pub device: Uuid,
}

impl Keeper {
    /// The keeper of the cache in `store`. A cache that cannot be read is deleted, and the
    /// daemon starts with none: it fetches its keys again.
    #[must_use]
    pub fn open(store: KeyStore) -> Self {
        let cache = store.load().unwrap_or_else(|error| {
            report::not_read(&error);
            if let Err(error) = store.forget() {
                report::not_deleted(&error);
            }
            KeyCache::default()
        });
        Self { cache, store }
    }

    /// The versions the cache holds: the `held` of the next batch.
    #[must_use]
    pub fn held(&self) -> Vec<KeyVersion> {
        self.cache.held()
    }

    /// Deletes the whole cache when no answer with `keys` came in the last 7 days (R-147).
    pub fn expire(&mut self, now: DateTime<Utc>) {
        if self.cache.is_stale(now) {
            report::expired();
            self.forget();
        }
    }

    /// Deletes the whole cache: after a refused token, and when the daemon leaves the machine.
    pub fn forget(&mut self) {
        self.cache.forget();
        if let Err(error) = self.store.forget() {
            report::not_deleted(&error);
        }
    }

    /// Follows the hub's `current` versions told at `now`: drops what is no longer current at
    /// once, stores that, then fetches each current version it lacks and stores again. A
    /// fetch that fails is tried at the next answer.
    pub async fn follow(&mut self, asking: Asking<'_>, current: &KeysCurrent, now: DateTime<Utc>) {
        let wanted = self.cache.apply(&current.current, now);
        self.save();
        if wanted.is_empty() {
            return;
        }
        for one in &wanted {
            let asked = KeyFetch {
                device: asking.device,
                name: one.name.clone(),
            };
            match asking.hub.fetch_key(asking.token, &asked).await {
                Answer::Accepted(fetched) => {
                    if !self.cache.keep(fetched, &wanted) {
                        report::fetch_failed(Cause::Unreadable);
                    }
                }
                // The hub refuses the token: the whole cache goes, as after a refused push
                // (K-E5 Section 2.1).
                Answer::Refused(RefusalCode::TokenRefused) => {
                    report::fetch_refused(RefusalCode::TokenRefused);
                    self.forget();
                    return;
                }
                Answer::Refused(code) => report::fetch_refused(code),
                Answer::Failed(cause) => report::fetch_failed(cause),
            }
        }
        self.save();
    }

    fn save(&self) {
        if let Err(error) = self.store.save(&self.cache) {
            report::not_stored(&error);
        }
    }
}

/// The keeper's log lines: by code or cause, never a key name or a value (LOG-007).
mod report {
    use daemon_core::wire::RefusalCode;

    use super::{Cause, KeyStoreError};

    pub fn not_read(error: &KeyStoreError) {
        tracing::error!(%error, "the key cache could not be read; it is deleted");
    }

    pub fn not_stored(error: &KeyStoreError) {
        tracing::error!(%error, "the key cache could not be stored");
    }

    pub fn not_deleted(error: &KeyStoreError) {
        tracing::error!(%error, "the key cache could not be deleted");
    }

    pub fn expired() {
        tracing::warn!("no answer of the hub named the keys for 7 days; the key cache is deleted");
    }

    pub fn fetch_refused(code: RefusalCode) {
        tracing::warn!(code = code.as_str(), "the hub refused a key fetch");
    }

    pub fn fetch_failed(cause: Cause) {
        tracing::warn!(%cause, "a key fetch failed; trying again at the next push");
    }
}
