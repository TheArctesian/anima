//! The answers of live reads held in hub memory (K-E1 Section 13.1). Pure: the time is a
//! parameter (CQ-021). Raw bodies only; nothing of it reaches the database or a log.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::Request;

use super::Keep;
use crate::ids::SourceAccountId;

/// How long a held answer may stand in for a failed refresh.
pub(super) const STALE_FOR: TimeDelta = TimeDelta::hours(1);
/// A read asked `fresh` still takes an answer younger than this.
pub(super) const FRESH_FLOOR: TimeDelta = TimeDelta::seconds(5);

/// How much the cache holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bounds {
    pub entries: usize,
    pub bytes: usize,
    /// No body larger than this is kept.
    pub body: usize,
}

/// The bounds of K-E1 Section 13.1: 2,000 entries, 64 MiB, no body over 512 KiB.
pub(super) const BOUNDS: Bounds = Bounds {
    entries: 2_000,
    bytes: 64 * 1024 * 1024,
    body: 512 * 1024,
};

/// What an answer is held under: the account, the path and the query, sorted.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Key {
    account: SourceAccountId,
    path: String,
    query: Vec<(String, String)>,
}

impl Key {
    pub(super) fn of(account: SourceAccountId, request: &Request) -> Self {
        let mut query = request.query.clone();
        query.sort();
        Self {
            account,
            path: request.path.clone(),
            query,
        }
    }
}

/// A held answer: a 2xx status, its body and when it was read.
#[derive(Clone)]
pub(super) struct Held {
    pub status: u16,
    pub body: Arc<[u8]>,
    pub at: DateTime<Utc>,
}

struct Entry {
    held: Held,
    fresh_until: DateTime<Utc>,
    used: u64,
}

/// The held answers, least recently used first out.
pub(super) struct Replies {
    bounds: Bounds,
    entries: HashMap<Key, Entry>,
    bytes: usize,
    clock: u64,
}

impl Default for Replies {
    fn default() -> Self {
        Self::new(BOUNDS)
    }
}

impl Replies {
    pub(super) fn new(bounds: Bounds) -> Self {
        Self {
            bounds,
            entries: HashMap::new(),
            bytes: 0,
            clock: 0,
        }
    }

    /// The answer under `key` when `keep` allows serving it at `now` without a new read: kept
    /// and inside its window, and, when `keep` asks for a fresh read, younger than five seconds.
    pub(super) fn fresh(&mut self, key: &Key, keep: Keep, now: DateTime<Utc>) -> Option<Held> {
        if !keep.is_kept() {
            return None;
        }
        let tick = self.tick();
        let entry = self.entries.get_mut(key)?;
        let young = now - entry.held.at < FRESH_FLOOR;
        let serves = now < entry.fresh_until && (!keep.is_forced() || young);
        serves.then(|| {
            entry.used = tick;
            entry.held.clone()
        })
    }

    /// The answer under `key` when it was read at most an hour before `now`: what a failed
    /// refresh serves, marked stale.
    pub(super) fn stale(&mut self, key: &Key, keep: Keep, now: DateTime<Utc>) -> Option<Held> {
        if !keep.is_kept() {
            return None;
        }
        let tick = self.tick();
        let entry = self.entries.get_mut(key)?;
        (now - entry.held.at <= STALE_FOR).then(|| {
            entry.used = tick;
            entry.held.clone()
        })
    }

    /// Holds `held` under `key` for as long as `keep` says. An answer `keep` never keeps, or
    /// one over the body bound, is not held, and the older answer under `key` goes too.
    pub(super) fn keep(&mut self, key: Key, keep: Keep, held: Held) {
        self.discard(&key);
        let Some(fresh_for) = keep.fresh_for(&held.body) else {
            return;
        };
        if held.body.len() > self.bounds.body {
            return;
        }
        let used = self.tick();
        self.bytes += held.body.len();
        let fresh_until = held.at + fresh_for;
        self.entries.insert(
            key,
            Entry {
                held,
                fresh_until,
                used,
            },
        );
        while self.entries.len() > self.bounds.entries || self.bytes > self.bounds.bytes {
            let Some(oldest) = self.least_recently_used() else {
                break;
            };
            self.discard(&oldest);
        }
    }

    /// Drops the answers of `account` whose path is `prefix` or lies below it.
    pub(super) fn forget(&mut self, account: SourceAccountId, prefix: &str) {
        let prefix = prefix.trim_end_matches('/');
        let below = |path: &str| {
            path.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        };
        let gone: Vec<Key> = self
            .entries
            .keys()
            .filter(|key| key.account == account && below(&key.path))
            .cloned()
            .collect();
        for key in gone {
            self.discard(&key);
        }
    }

    /// How many answers are held, and their bytes.
    #[cfg(test)]
    pub(super) fn size(&self) -> (usize, usize) {
        (self.entries.len(), self.bytes)
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    fn least_recently_used(&self) -> Option<Key> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| key.clone())
    }

    /// Drops the answer under `key`: a newer reply said something else.
    pub(super) fn discard(&mut self, key: &Key) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes -= entry.held.body.len();
        }
    }
}
