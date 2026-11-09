//! The domain expiry probe (spec 6.4, CHK-009, D-29): the registration expiry of a domain,
//! read over RDAP.
//!
//! The IANA bootstrap file names the RDAP service of each top-level domain; [`Rdap`] keeps the
//! last one read for 24 hours. The probe asks that service for `domain/<name>` and reads the
//! `expiration` event. Every connection leaves through [`egress`]: HTTPS only, each host
//! resolved once through `targets::resolve_checked` and reached only at the checked
//! addresses, TLS always verified, at most 3 redirects each checked again as a new target, and
//! the body read only up to a cap (P3-003, P-6). The whole probe ends at the check's time
//! limit.
//!
//! The hub could not test the domain when RDAP cannot be reached or read, so that result is
//! Unknown with a fixed message, never Down. Down comes only from the registry: the domain
//! has expired, or the registry has no record of it. No message holds a URL, an address or
//! any of the reply (LOG-007).

mod bootstrap;
mod cache;
mod egress;
mod expiry;
mod reply;
#[cfg(test)]
pub(in crate::checks) mod tests;

use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::Url;

use super::outcome::{Failure, ProbeResult};
use crate::targets::{AddressPolicy, TargetError};
use bootstrap::Bootstrap;
use cache::Cache;
use egress::{Egress, FetchError};

/// Where IANA publishes the RDAP bootstrap file for domain names (RFC 9224).
const BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/dns.json";
/// The most bootstrap bytes read; the file is about 40 KiB.
const BOOTSTRAP_CAP: usize = 1024 * 1024;
/// The most bytes of one RDAP reply read.
const REPLY_CAP: usize = 256 * 1024;

/// One domain expiry check's settings, as the probe needs them.
#[derive(Clone, Copy, Debug)]
pub(in crate::checks) struct Lookup<'a> {
    pub(in crate::checks) domain: &'a str,
    pub(in crate::checks) warn_days: u32,
    pub(in crate::checks) timeout: Duration,
}

/// What every domain expiry probe shares: the egress rules and the cached bootstrap file.
/// One lives in the scheduler, so the cache outlives each attempt; it is never global.
#[derive(Debug)]
pub(in crate::checks) struct Rdap {
    egress: Egress,
    bootstrap_url: Url,
    cache: Cache,
}

impl Rdap {
    /// The probe state that reaches only addresses `policy` permits.
    pub(in crate::checks) fn new(policy: AddressPolicy) -> Self {
        Self {
            egress: Egress::new(policy),
            bootstrap_url: Url::parse(BOOTSTRAP_URL).expect("the bootstrap address is a URL"),
            cache: Cache::default(),
        }
    }

    /// A probe state for local test servers: the bootstrap file at `bootstrap_url`, and the
    /// DER root certificate `root` trusted besides the built-in roots.
    #[cfg(test)]
    pub(in crate::checks) fn for_tests(
        policy: AddressPolicy,
        root: Vec<u8>,
        bootstrap_url: Url,
    ) -> Self {
        Self {
            egress: Egress::new(policy).trusting(root),
            bootstrap_url,
            cache: Cache::default(),
        }
    }

    /// Reads the domain's expiry and judges it at `now`, within the check's time limit
    /// (D-29).
    pub(in crate::checks) async fn observe(
        &self,
        lookup: Lookup<'_>,
        now: DateTime<Utc>,
    ) -> ProbeResult {
        let domain = lookup.domain.to_ascii_lowercase();
        match tokio::time::timeout(lookup.timeout, self.expiry(&domain, now)).await {
            Ok(Ok(not_after)) => expiry::judge(not_after, lookup.warn_days, now),
            Ok(Err(Failure::DomainNotFound)) => ProbeResult::failed(&Failure::DomainNotFound),
            Ok(Err(failure)) => ProbeResult::unjudged(&failure),
            Err(_) => ProbeResult::unjudged(&Failure::TimedOut),
        }
    }

    /// The expiry date the domain's RDAP service gives; `None` when it gives none.
    async fn expiry(
        &self,
        domain: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, Failure> {
        let bootstrap = self.bootstrap(now).await?;
        let query = bootstrap.query(domain)?;
        let body = match self.egress.get(&query, REPLY_CAP).await {
            Err(FetchError::Status(404)) => return Err(Failure::DomainNotFound),
            fetched => fetched.map_err(failure)?,
        };
        reply::expiration(&body).map_err(|_| Failure::RdapUnreadable)
    }

    /// The bootstrap file kept since less than 24 hours before `now`, or a fresh read.
    async fn bootstrap(&self, now: DateTime<Utc>) -> Result<Arc<Bootstrap>, Failure> {
        if let Some(kept) = self.cache.fresh(now) {
            return Ok(kept);
        }
        let body = self
            .egress
            .get(&self.bootstrap_url, BOOTSTRAP_CAP)
            .await
            .map_err(failure)?;
        let read = Arc::new(Bootstrap::parse(&body).map_err(|_| Failure::RdapUnreadable)?);
        self.cache.keep(now, Arc::clone(&read));
        Ok(read)
    }
}

/// The probe failure a refused or failed connection stands for.
fn failure(error: FetchError) -> Failure {
    match error {
        FetchError::Target(TargetError::Refused) => Failure::NotPermitted,
        FetchError::Target(TargetError::Unresolved) => Failure::Unresolved,
        FetchError::Address => Failure::RdapNotHttps,
        FetchError::Redirects => Failure::TooManyRedirects,
        FetchError::Status(code) => Failure::Status(code),
        FetchError::TooLong => Failure::RdapTooLong,
        FetchError::Tls => Failure::Handshake,
        FetchError::Setup | FetchError::Connect | FetchError::Exchange => Failure::Request,
    }
}
