//! The daemon's copy of the operational keys of its device, and the rules that keep it a copy
//! (K-E5 Section 2.1, R-147). Pure: the time is a parameter (CQ-021).
//!
//! The cache holds one version of each key. After each answer of the hub that had `keys`, it
//! drops each key that is not in `current` and each key whose version is not the current one,
//! and asks for the current versions it does not hold. When the last such answer is more than
//! 7 days old, the whole cache goes. Nothing here implements `Debug`: a cache holds values.

use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use daemon_core::wire::{FetchedKey, KeyValue, KeyVersion, versions_fit};

/// How long the cache lives without an answer of the hub that had `keys` (R-147).
pub const CACHE_LIFE: TimeDelta = TimeDelta::days(7);

/// The cached keys by name, and the time of the last answer of the hub that had `keys`.
#[derive(Default)]
pub struct KeyCache {
    keys: BTreeMap<String, FetchedKey>,
    answered_at: Option<DateTime<Utc>>,
}

impl KeyCache {
    /// A cache of `keys`, last told the current versions at `answered_at`.
    #[must_use]
    pub fn of(keys: Vec<FetchedKey>, answered_at: Option<DateTime<Utc>>) -> Self {
        Self {
            keys: keys
                .into_iter()
                .map(|key| (key.name.clone(), key))
                .collect(),
            answered_at,
        }
    }

    /// The version of each cached key, by name: the `held` of a push.
    #[must_use]
    pub fn held(&self) -> Vec<KeyVersion> {
        self.keys
            .values()
            .map(|key| KeyVersion {
                name: key.name.clone(),
                version: key.version,
            })
            .collect()
    }

    /// Takes the hub's `current` list, told at `now`: drops each key that is not in it or is at
    /// another version, and returns the versions to fetch. A list that breaks the rules of a
    /// list (too long, a name twice, a name that is not a key name) is ignored whole, and the
    /// cache stays as it was.
    pub fn apply(&mut self, current: &[KeyVersion], now: DateTime<Utc>) -> Vec<KeyVersion> {
        if !versions_fit(current) {
            return Vec::new();
        }
        self.keys.retain(|name, key| {
            current
                .iter()
                .any(|one| &one.name == name && one.version == key.version)
        });
        self.answered_at = Some(now);
        current
            .iter()
            .filter(|one| !self.keys.contains_key(&one.name))
            .cloned()
            .collect()
    }

    /// Keeps `fetched` when it is a version the hub named in `wanted`; whether it was kept.
    pub fn keep(&mut self, fetched: FetchedKey, wanted: &[KeyVersion]) -> bool {
        let named = wanted
            .iter()
            .any(|one| one.name == fetched.name && one.version == fetched.version);
        if named {
            self.keys.insert(fetched.name.clone(), fetched);
        }
        named
    }

    /// Whether the whole cache must go at `now`: it holds keys and no answer with `keys` came
    /// in the last 7 days.
    #[must_use]
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        !self.keys.is_empty()
            && self
                .answered_at
                .is_none_or(|answered| now - answered > CACHE_LIFE)
    }

    /// Drops every key.
    pub fn forget(&mut self) {
        self.keys.clear();
    }

    /// The value of the key named `name`, when the cache holds it.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&KeyValue> {
        self.keys.get(name).map(|key| &key.value)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The cached keys, by name.
    pub fn keys(&self) -> impl Iterator<Item = &FetchedKey> {
        self.keys.values()
    }

    #[must_use]
    pub const fn answered_at(&self) -> Option<DateTime<Utc>> {
        self.answered_at
    }
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::*;

    fn at(hour: i64) -> DateTime<Utc> {
        DateTime::UNIX_EPOCH + TimeDelta::hours(hour)
    }

    fn version(name: &str, version: u32) -> KeyVersion {
        KeyVersion {
            name: name.to_owned(),
            version,
        }
    }

    fn fetched(name: &str, version: u32, value: &str) -> FetchedKey {
        FetchedKey {
            name: name.to_owned(),
            version,
            value: KeyValue::new(Zeroizing::new(value.as_bytes().to_vec())).unwrap(),
        }
    }

    fn value_of<'a>(cache: &'a KeyCache, name: &str) -> Option<&'a [u8]> {
        cache.value(name).map(KeyValue::expose)
    }

    #[test]
    fn r_147_a_key_that_is_no_longer_current_is_dropped() {
        let mut cache = KeyCache::of(vec![fetched("a", 1, "one"), fetched("b", 1, "bee")], None);
        let wanted = cache.apply(&[version("b", 1)], at(0));
        assert_eq!(wanted, Vec::<KeyVersion>::new());
        assert_eq!(value_of(&cache, "a"), None, "revoked or ungranted: dropped");
        assert_eq!(value_of(&cache, "b"), Some(&b"bee"[..]));
    }

    #[test]
    fn r_147_a_new_version_drops_the_old_one_and_is_fetched() {
        let mut cache = KeyCache::of(vec![fetched("a", 1, "one")], None);
        let wanted = cache.apply(&[version("a", 2), version("c", 1)], at(0));
        assert_eq!(wanted, [version("a", 2), version("c", 1)]);
        assert_eq!(
            value_of(&cache, "a"),
            None,
            "the superseded value is gone at once"
        );
        assert!(
            !cache.keep(fetched("a", 1, "one"), &wanted),
            "an old version is refused"
        );
        assert!(cache.keep(fetched("a", 2, "two"), &wanted));
        assert_eq!(cache.held(), [version("a", 2)]);
    }

    #[test]
    fn r_147_an_unreadable_current_list_changes_nothing() {
        let mut cache = KeyCache::of(vec![fetched("a", 1, "one")], Some(at(0)));
        let twice = [version("a", 2), version("a", 3)];
        assert_eq!(cache.apply(&twice, at(5)), Vec::<KeyVersion>::new());
        assert_eq!(cache.held(), [version("a", 1)]);
        assert_eq!(cache.answered_at(), Some(at(0)));
    }

    #[test]
    fn r_147_the_cache_goes_after_7_days_without_an_answer_with_keys() {
        let mut cache = KeyCache::of(vec![fetched("a", 1, "one")], None);
        cache.apply(&[version("a", 1)], at(0));
        assert!(!cache.is_stale(at(7 * 24)), "exactly 7 days");
        assert!(cache.is_stale(at(7 * 24 + 1)));
        cache.apply(&[version("a", 1)], at(7 * 24));
        assert!(!cache.is_stale(at(7 * 24 + 1)), "an answer renews it");
    }

    #[test]
    fn r_147_a_cache_with_no_answer_time_is_stale() {
        let cache = KeyCache::of(vec![fetched("a", 1, "one")], None);
        assert!(cache.is_stale(at(0)));
        assert!(
            !KeyCache::default().is_stale(at(0)),
            "an empty cache has nothing to drop"
        );
    }
}
