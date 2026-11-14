//! Installation tokens, held in hub memory only (PRJ-018, K-E1 Section 7).
//!
//! A token is never written to the database, a log, an audit entry or an error, and has no
//! `Debug`. The map's lock is never held across an `await`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::Minted;

use crate::{fleet::credentials::Secret, ids::SourceAccountId};

/// A token with less than this left is minted again.
const REMINT_BEFORE: TimeDelta = TimeDelta::minutes(5);

struct Held {
    token: Arc<Secret>,
    expires_at: DateTime<Utc>,
}

/// The installation token of each account, in memory. Cheap to clone; clones share the map.
#[derive(Clone, Default)]
pub struct Tokens(Arc<Mutex<HashMap<SourceAccountId, Held>>>);

impl Tokens {
    fn with<T>(&self, f: impl FnOnce(&mut HashMap<SourceAccountId, Held>) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The account's token when more than five minutes of it are left at `now`.
    #[must_use]
    pub fn fresh(&self, account: SourceAccountId, now: DateTime<Utc>) -> Option<Arc<Secret>> {
        self.with(|held| {
            held.get(&account)
                .filter(|h| h.expires_at - now > REMINT_BEFORE)
                .map(|h| h.token.clone())
        })
    }

    /// Keeps `minted` as the account's token and returns it.
    #[must_use]
    pub fn keep(&self, account: SourceAccountId, minted: Minted) -> Arc<Secret> {
        let token = Arc::new(Secret::from(minted.token));
        let kept = Held {
            token: token.clone(),
            expires_at: minted.expires_at,
        };
        self.with(|held| held.insert(account, kept));
        token
    }

    /// Drops the account's token: it was refused, or the account was removed.
    pub fn forget(&self, account: SourceAccountId) {
        self.with(|held| held.remove(&account));
    }
}
