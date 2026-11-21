//! The certificates of the routes (Section 12.5, SF-NEW-routeacme): the interface of an ACME
//! issuer, the rules of when the hub asks, and the context that binds a sealed key to its route.
//!
//! The hub holds the ACME account and asks; the proxy only answers the challenge and gets the
//! certificate from the hub. No real ACME request is made in this version: [`NotConfigured`] is
//! the one issuer of the hub until OD-HOST-44 is decided, and the tests give a double.

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};

use super::names::RouteName;
use crate::{fleet::credentials::Secret, ids::RouteId};

/// The authority that the CAA check names.
pub const AUTHORITY: &str = "letsencrypt.org";
/// A certificate with less time than this left gets a new order.
const RENEW_BEFORE: TimeDelta = TimeDelta::days(30);
/// The first wait after a failed order; each failure doubles it, up to [`WAIT_MAX`].
const WAIT_FIRST: TimeDelta = TimeDelta::minutes(15);
const WAIT_MAX: TimeDelta = TimeDelta::days(2);

/// A certificate that an issuer gave: the chain in PEM, the private key in PEM and the end of
/// its validity. No `Debug`: it holds the key (LOG-007).
pub struct Issued {
    pub chain_pem: String,
    pub key_pem: Secret,
    pub not_after: DateTime<Utc>,
}

/// Why an issuer gave no certificate. Holds no key and no answer of the authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IssueError {
    #[error("no ACME issuer is configured on this hub")]
    NotConfigured,
    #[error("the authority refused the order")]
    Refused,
    #[error("the authority did not answer")]
    Unavailable,
}

/// An ACME issuer: asks the authority for a certificate of one name.
#[async_trait]
pub trait Issuer: Send + Sync {
    async fn issue(&self, name: &RouteName) -> Result<Issued, IssueError>;
}

/// The issuer of a hub that has no ACME account yet: it refuses each order.
pub struct NotConfigured;

#[async_trait]
impl Issuer for NotConfigured {
    async fn issue(&self, _name: &RouteName) -> Result<Issued, IssueError> {
        Err(IssueError::NotConfigured)
    }
}

/// Whether a route needs a new order at `now`: it has no certificate, or one with less than
/// 30 days left, and the wait after its last failure is over.
#[must_use]
pub fn needs_order(
    not_after: Option<DateTime<Utc>>,
    next_attempt: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    let due = not_after.is_none_or(|end| end - now < RENEW_BEFORE);
    due && next_attempt.is_none_or(|next| next <= now)
}

/// When the hub may ask again after `failures` failed orders in a row, the last at `now`.
#[must_use]
pub fn next_attempt(failures: u32, now: DateTime<Utc>) -> DateTime<Utc> {
    let doublings = failures.saturating_sub(1).min(16);
    let wait = WAIT_FIRST * 2_i32.pow(doublings);
    now + wait.min(WAIT_MAX)
}

/// The context that binds a sealed key to its route and name, so a key moved to a different
/// row does not open (SEC-003).
#[must_use]
pub fn key_context(route: RouteId, name: &RouteName) -> String {
    format!("anima/routes/certificate-key/v1/{route}/{name}")
}
