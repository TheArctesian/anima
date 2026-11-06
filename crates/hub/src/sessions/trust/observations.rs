//! Host keys the hub saw in a connection test and the operator has not accepted yet (D-5):
//! kept 15 minutes per host and port, so test, save and `acceptHostKey` works for a new
//! device. Pure; time comes in as a parameter (CQ-021).

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, TimeDelta, Utc};

use crate::sessions::public_key::PublicKey;

/// How long an observation stays after the hub last saw the key.
pub(in crate::sessions) const LIFETIME: TimeDelta = TimeDelta::minutes(15);

/// Most observations held. Connection tests come only from signed-in operators, so this
/// bounds memory even when one tests many hosts in a loop.
pub(in crate::sessions) const MAX_OBSERVATIONS: usize = 256;

/// A host and an SSH port, in one spelling: a name is trimmed and in lower case; an IP
/// literal is in its canonical text. So a test of `PVE-01.lan` and a device saved as
/// `pve-01.lan` share their observation. A final dot is kept: `pve-01.lan.` is absolute and
/// `pve-01.lan` may resolve through a search domain, so they can name different hosts and
/// never share a key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::sessions) struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    pub(in crate::sessions) fn new(host: &str, port: u16) -> Self {
        let host = host.trim();
        let host = host
            .parse::<IpAddr>()
            .map_or_else(|_| host.to_ascii_lowercase(), |ip| ip.to_string());
        Self { host, port }
    }
}

/// A key seen at an endpoint: when the hub first saw this key there, and when last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Observation {
    pub(in crate::sessions) key: PublicKey,
    pub(in crate::sessions) first_seen_at: DateTime<Utc>,
    pub(in crate::sessions) last_seen_at: DateTime<Utc>,
}

impl Observation {
    /// Gone at `now` once [`LIFETIME`] has passed since the key was last seen.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.last_seen_at + LIFETIME
    }
}

/// The latest key seen at each endpoint, at most [`MAX_OBSERVATIONS`].
#[derive(Debug, Default)]
pub(in crate::sessions) struct Observations(HashMap<Endpoint, Observation>);

impl Observations {
    /// Records that `endpoint` presented `key` at `now` and returns the observation. The
    /// same key again keeps its first-seen time; another key replaces it. Expired
    /// observations go first; when the map is still full, the one seen longest ago goes.
    pub(in crate::sessions) fn record(
        &mut self,
        endpoint: Endpoint,
        key: PublicKey,
        now: DateTime<Utc>,
    ) -> Observation {
        self.0.retain(|_, seen| !seen.is_expired(now));
        let first_seen_at = match self.0.get(&endpoint) {
            Some(seen) if seen.key == key => seen.first_seen_at,
            _ => now,
        };
        if !self.0.contains_key(&endpoint) && self.0.len() >= MAX_OBSERVATIONS {
            self.forget_oldest();
        }
        let observation = Observation {
            key,
            first_seen_at,
            last_seen_at: now,
        };
        self.0.insert(endpoint, observation.clone());
        observation
    }

    /// The key last seen at `endpoint`, unless it expired at `now`.
    pub(in crate::sessions) fn find(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation> {
        self.0
            .get(endpoint)
            .filter(|seen| !seen.is_expired(now))
            .cloned()
    }

    fn forget_oldest(&mut self) {
        let oldest = self
            .0
            .iter()
            .min_by_key(|(_, seen)| seen.last_seen_at)
            .map(|(endpoint, _)| endpoint.clone());
        if let Some(endpoint) = oldest {
            self.0.remove(&endpoint);
        }
    }

    #[cfg(test)]
    pub(in crate::sessions) fn len(&self) -> usize {
        self.0.len()
    }
}
