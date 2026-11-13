//! SNMP probe (D-5, D-13): a GET of one OID over UDP, v2c with a community or v3 authPriv
//! (HMAC-SHA-256-192, AES-128-CFB).
//!
//! The device host is resolved once and only an address the policy permits is used. One
//! socket is bound per attempt and only replies from the target's address and port with the
//! request's ids are read. The community and passwords go only into the request; no message,
//! log or error holds them or anything the agent sent (LOG-007).

pub(in crate::fleet::checks) mod ber;
pub(in crate::fleet::checks) mod community;
pub(in crate::fleet::checks) mod link;
pub(in crate::fleet::checks) mod message;
pub(in crate::fleet::checks) mod pdu;
pub(in crate::fleet::checks) mod usm;

use std::{net::SocketAddr, time::Duration};

use zeroize::Zeroizing;

use self::{
    link::Link,
    message::{Engine, Ids},
    pdu::Pdu,
    usm::{Keys, SALT_LEN},
};
use super::{
    ProbeEnv, ProbeTarget,
    outcome::{Failure, ProbeResult},
    resolve::allowed_addresses,
};
use crate::fleet::checks::config::{SnmpConfig, SnmpSecrets, oid_arcs};

/// The random values of one attempt: the ids of each request and the AES salt.
#[derive(Clone, Copy, Debug)]
pub(in crate::fleet::checks) struct Draws {
    pub(in crate::fleet::checks) discovery: Ids,
    pub(in crate::fleet::checks) get: Ids,
    pub(in crate::fleet::checks) salt: [u8; SALT_LEN],
}

/// How many random bytes one attempt's [`Draws`] take.
pub(in crate::fleet::checks) const DRAWS_LEN: usize = 16 + SALT_LEN;

impl Draws {
    /// Fresh values from the operating system's random source: the I/O edge of the randomness.
    ///
    /// # Errors
    /// The random source's error when it yields no bytes.
    pub(in crate::fleet::checks) fn fresh() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; DRAWS_LEN];
        getrandom::fill(&mut bytes)?;
        Ok(Self::from_bytes(&bytes))
    }

    /// The values these random bytes give: four 31-bit ids from the first 16, then the salt.
    pub(in crate::fleet::checks) fn from_bytes(bytes: &[u8; DRAWS_LEN]) -> Self {
        let id = |at: usize| {
            let raw = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            i32::try_from(raw >> 1).unwrap_or(0)
        };
        let mut salt = [0_u8; SALT_LEN];
        salt.copy_from_slice(&bytes[16..]);
        Self {
            discovery: Ids {
                message: id(0),
                request: id(4),
            },
            get: Ids {
                message: id(8),
                request: id(12),
            },
            salt,
        }
    }
}

/// One attempt of the SNMP check, within its time limit, with the attempt's random values
/// given (CQ-021): the caller draws them at its I/O edge.
pub(in crate::fleet::checks) async fn observe(
    target: &ProbeTarget,
    (config, secrets): (&SnmpConfig, &SnmpSecrets),
    env: &ProbeEnv,
    draws: Draws,
) -> ProbeResult {
    if config.validate().is_err() {
        return ProbeResult::failed(&Failure::BadConfig);
    }
    let limit = Duration::from_millis(u64::from(config.timeout_ms));
    let attempt = async {
        match get(target, (config, secrets), env, draws).await {
            Ok((ip, elapsed)) => ProbeResult::up(elapsed).via(ip.ip()),
            Err(failure) => ProbeResult::failed(&failure),
        }
    };
    tokio::time::timeout(limit, attempt)
        .await
        .unwrap_or_else(|_| ProbeResult::failed(&Failure::TimedOut))
}

/// Resolves the target, asks for the OID and judges the answer.
async fn get(
    target: &ProbeTarget,
    (config, secrets): (&SnmpConfig, &SnmpSecrets),
    env: &ProbeEnv,
    draws: Draws,
) -> Result<(SocketAddr, Duration), Failure> {
    let oid = oid_arcs(&config.oid).ok_or(Failure::BadConfig)?;
    if secrets.version() != config.version {
        return Err(Failure::SecretsUnbound);
    }
    let ip = *allowed_addresses(&target.host, None, env)
        .await?
        .first()
        .ok_or(Failure::NotPermitted)?;
    let peer = SocketAddr::new(ip, config.port);
    let mut link = Link::open(peer).await?;
    let (pdu, elapsed) = match secrets {
        SnmpSecrets::Community(community) => {
            let request = community::request(community.expose(), &oid, draws.get.request)
                .ok_or(Failure::BadConfig)?;
            let request_id = draws.get.request;
            link.exchange(&request, |reply| community::response(reply, request_id))
                .await?
        }
        SnmpSecrets::User { auth, privacy } => {
            let user = config.username.as_deref().unwrap_or_default().as_bytes();
            let passwords = (auth.expose(), privacy.expose());
            v3(&mut link, user, passwords, (&oid, draws)).await?
        }
    };
    pdu::judge(&pdu, &oid, config.expected.as_deref())?;
    Ok((peer, elapsed))
}

/// The localised keys, derived on the blocking pool: two SHA-256 passes over 1 MiB would
/// otherwise hold an async worker for milliseconds. The copies are zeroized on drop, also
/// when the attempt's time limit drops the wait first.
async fn localised(auth: &[u8], privacy: &[u8], engine_id: &[u8]) -> Result<Keys, Failure> {
    let auth = Zeroizing::new(auth.to_vec());
    let privacy = Zeroizing::new(privacy.to_vec());
    let engine_id = engine_id.to_vec();
    tokio::task::spawn_blocking(move || Keys::localised(&auth, &privacy, &engine_id))
        .await
        .ok()
        .flatten()
        .ok_or(Failure::SecretMissing)
}

/// Discovers the engine, localises the keys to it, and sends the authPriv `GetRequest`.
async fn v3(
    link: &mut Link,
    user: &[u8],
    (auth, privacy): (&[u8], &[u8]),
    (oid, draws): (&[u32], Draws),
) -> Result<(Pdu, Duration), Failure> {
    let discovery = message::discovery(draws.discovery);
    let (engine, _): (Engine, _) = link
        .exchange(&discovery, |reply| message::engine(reply, draws.discovery))
        .await?;
    let keys = localised(auth, privacy, &engine.id).await?;
    let request = message::get((&keys, &engine), user, oid, (draws.get, draws.salt))
        .ok_or(Failure::BadConfig)?;
    link.exchange(&request, |reply| {
        message::response(reply, (&keys, &engine), user, draws.get)
    })
    .await
}
