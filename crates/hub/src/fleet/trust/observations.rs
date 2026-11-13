//! What a device presented in a connection test and the operator has not accepted yet: an
//! SSH host key or an RDP server certificate, kept 15 minutes per host and port, so test, save
//! and `acceptHostKey` or `acceptServerCertificate` works for a new device (SEC-006, SEC-007).
//! Pure; time comes in as a parameter (CQ-021).

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, TimeDelta, Utc};

/// How long an observation stays after the hub last saw what was presented.
pub(in crate::fleet::trust) const LIFETIME: TimeDelta = TimeDelta::minutes(15);

/// Most observations held. Connection tests come only from signed-in operators, so this
/// bounds memory even when one tests many hosts in a loop.
pub(in crate::fleet::trust) const MAX_OBSERVATIONS: usize = 256;

/// A host and a port, in one spelling: a name is trimmed and in lower case; an IP
/// literal is in its canonical text. So a test of `PVE-01.lan` and a device saved as
/// `pve-01.lan` share their observation. A final dot is kept: `pve-01.lan.` is absolute and
/// `pve-01.lan` may resolve through a search domain, so they can name different hosts and
/// never share an observation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    pub(crate) fn new(host: &str, port: u16) -> Self {
        let host = host.trim();
        let host = host
            .parse::<IpAddr>()
            .map_or_else(|_| host.to_ascii_lowercase(), |ip| ip.to_string());
        Self { host, port }
    }
}

/// What an endpoint presented (a host key or a certificate): when the hub first saw it there,
/// and when last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observation<T> {
    pub(crate) presented: T,
    pub(crate) first_seen_at: DateTime<Utc>,
    pub(crate) last_seen_at: DateTime<Utc>,
}

impl<T> Observation<T> {
    /// Gone at `now` once [`LIFETIME`] has passed since it was last seen.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.last_seen_at + LIFETIME
    }
}

/// The latest thing seen at each endpoint, at most [`MAX_OBSERVATIONS`].
#[derive(Debug)]
pub(in crate::fleet::trust) struct Observations<T>(HashMap<Endpoint, Observation<T>>);

impl<T> Default for Observations<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: Clone + PartialEq> Observations<T> {
    /// Records that `endpoint` presented `presented` at `now` and returns the observation.
    /// The same thing again keeps its first-seen time; another replaces it. Expired
    /// observations go first; when the map is still full, the one seen longest ago goes.
    pub(in crate::fleet::trust) fn record(
        &mut self,
        endpoint: Endpoint,
        presented: T,
        now: DateTime<Utc>,
    ) -> Observation<T> {
        self.0.retain(|_, seen| !seen.is_expired(now));
        let first_seen_at = match self.0.get(&endpoint) {
            Some(seen) if seen.presented == presented => seen.first_seen_at,
            _ => now,
        };
        if !self.0.contains_key(&endpoint) && self.0.len() >= MAX_OBSERVATIONS {
            self.forget_oldest();
        }
        let observation = Observation {
            presented,
            first_seen_at,
            last_seen_at: now,
        };
        self.0.insert(endpoint, observation.clone());
        observation
    }

    /// What was last seen at `endpoint`, unless it expired at `now`.
    pub(in crate::fleet::trust) fn find(
        &self,
        endpoint: &Endpoint,
        now: DateTime<Utc>,
    ) -> Option<Observation<T>> {
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
    pub(in crate::fleet::trust) fn len(&self) -> usize {
        self.0.len()
    }
}
