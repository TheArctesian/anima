//! The proof that a name belongs to this hub (Section 12.4 of the design, SF-NEW-routetoken).
//!
//! A hosted hub shares the address of the door, so it proves a name with a `TXT` record
//! `_anima.<name>` that holds its token: 256 random bits, bound to the pair of the hub and the
//! name. A hub that a person hosts is the only hub at its address, so the name proves itself
//! when each address it resolves to is the public address of the hub. One failed read changes
//! nothing: a proven name is lost only after [`LOST_AFTER`] failed reads in a row. The reads are
//! I/O behind [`Resolver`]; the rest is pure.

use std::{fmt::Write as _, net::IpAddr};

use async_trait::async_trait;

use super::names::RouteName;

/// Failed reads in a row after which a proven name is lost.
pub const LOST_AFTER: u32 = 3;
/// The random bytes of a token.
const TOKEN_BYTES: usize = 32;

/// How this hub proves a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofRule {
    /// A `TXT` record with the token, for a hub behind the shared door. `hub` is the hub's name.
    Token { hub: String },
    /// The name resolves only to this address: a hub that a person hosts.
    Address(IpAddr),
}

/// The state of the proof of a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proof {
    /// Not proven yet.
    Waiting,
    Proven,
    /// It was proven, and then failed [`LOST_AFTER`] reads in a row.
    Lost,
}

impl Proof {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Proven => "proven",
            Self::Lost => "lost",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        match text {
            "waiting" => Some(Self::Waiting),
            "proven" => Some(Self::Proven),
            "lost" => Some(Self::Lost),
            _ => None,
        }
    }
}

/// What one read of DNS found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
    /// The record names this hub and this token, or the addresses are the hub's.
    Holds,
    /// The record or the address is not there or is different.
    DoesNotHold,
    /// DNS did not answer.
    Failed,
}

/// Why DNS gave no answer. Holds no name (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the DNS lookup failed")]
pub struct LookupFailed;

/// One `CAA` record: its tag (`issue`, `issuewild`, `iodef`) and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caa {
    pub tag: String,
    pub value: String,
}

/// The DNS reads of the routes. The real one asks a resolver; tests give a double.
#[async_trait]
pub trait Resolver: Send + Sync {
    /// The texts of the `TXT` records of `name`; empty when it has none.
    async fn txt(&self, name: &str) -> Result<Vec<String>, LookupFailed>;
    /// The addresses of `name`; empty when it has none.
    async fn addresses(&self, name: &str) -> Result<Vec<IpAddr>, LookupFailed>;
    /// The `CAA` records of exactly `name`, not of a domain above it.
    async fn caa(&self, name: &str) -> Result<Vec<Caa>, LookupFailed>;
}

/// A new token: 256 random bits in lower-case hex.
///
/// # Errors
/// When the random source of the system gives nothing.
pub fn new_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes)?;
    Ok(bytes
        .iter()
        .fold(String::with_capacity(TOKEN_BYTES * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        }))
}

/// The name of the `TXT` record of `name`.
#[must_use]
pub fn record_name(name: &RouteName) -> String {
    format!("_anima.{name}")
}

/// The text of the `TXT` record: the hub and the token together, so a token copied to a
/// different hub proves nothing.
#[must_use]
pub fn record_value(hub: &str, token: &str) -> String {
    format!("anima-route hub={hub} token={token}")
}

/// Reads DNS once for `name` by `rule`.
pub async fn read(
    resolver: &dyn Resolver,
    rule: &ProofRule,
    name: &RouteName,
    token: &str,
) -> Read {
    match rule {
        ProofRule::Token { hub } => match resolver.txt(&record_name(name)).await {
            Ok(texts) => holds(
                texts
                    .iter()
                    .any(|text| text.trim() == record_value(hub, token)),
            ),
            Err(LookupFailed) => Read::Failed,
        },
        ProofRule::Address(public) => match resolver.addresses(name.as_str()).await {
            Ok(found) => {
                holds(!found.is_empty() && found.iter().all(|ip| ip.to_canonical() == *public))
            }
            Err(LookupFailed) => Read::Failed,
        },
    }
}

const fn holds(yes: bool) -> Read {
    if yes { Read::Holds } else { Read::DoesNotHold }
}

/// The state after one read, from the state and the failed reads in a row before it. Returns
/// the new state and the new count of failed reads.
#[must_use]
pub const fn after(proof: Proof, failed_reads: u32, read: Read) -> (Proof, u32) {
    match (proof, read) {
        (_, Read::Holds) => (Proof::Proven, 0),
        (Proof::Proven, _) if failed_reads + 1 >= LOST_AFTER => (Proof::Lost, failed_reads + 1),
        (state, _) => (state, failed_reads.saturating_add(1)),
    }
}
