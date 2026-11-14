//! One request of a run: the address check, the token, the send, and the verdict on access
//! before any provider reads the reply (K-E1 Section 7).

use std::sync::Arc;

use ring::rand::SystemRandom;
use source_exchange::{Declaration, Request};

use super::{Claim, Failure};
use crate::{
    engine::sources::{
        Scheduling,
        acquisition::{
            MintFailure, mint, stored_key,
            transport::{Answer, Fault},
            verdict::{Verdict, judge},
        },
        model::ErrorKind,
    },
    fleet::credentials::{self, Secret, load_source_secret},
};

const REFUSED_PATH: Failure = Failure::new(
    ErrorKind::Provider,
    "The source asked for an address the hub does not send to.",
);
const REFUSED_KEY: Failure = Failure::new(
    ErrorKind::Auth,
    "The source refused the key. Connect the account again.",
);

/// Fetches `request` for the claimed account: refused before anything is sent when its
/// address could leave the source's API base; a 401 drops the token and is `auth`; a rate
/// limit is `rate_limit` until its end. Any other reply is the provider's to read.
pub(super) async fn fetch(
    scheduling: &Scheduling,
    (claim, declaration): (&Claim, &Declaration),
    request: &Request,
) -> Result<Answer, Failure> {
    let url = scheduling
        .transport
        .url(declaration.api_base, request)
        .map_err(|_| REFUSED_PATH)?;
    let token = token(scheduling, claim, declaration).await?;
    let answer = scheduling
        .transport
        .get(request, url, &token)
        .await
        .map_err(|fault| match fault {
            Fault::Refused => REFUSED_PATH,
            Fault::Network => Failure::new(ErrorKind::Network, "The source could not be reached."),
            Fault::TooLarge => Failure::new(
                ErrorKind::Provider,
                "The source sent a reply larger than the hub reads.",
            ),
        })?;
    match judge(answer.status, answer.limits, scheduling.clock.now()) {
        Verdict::Usable => Ok(answer),
        Verdict::Refused => {
            scheduling.transport.tokens().forget(claim.account.id);
            Err(REFUSED_KEY)
        }
        Verdict::RateLimited(until) => Err(rate_limited(until)),
    }
}

fn rate_limited(until: chrono::DateTime<chrono::Utc>) -> Failure {
    Failure {
        until: Some(until),
        ..Failure::new(
            ErrorKind::RateLimit,
            "The source limits requests; the hub waits.",
        )
    }
}

/// The account's installation token: the one in memory while more than five minutes of it
/// are left, otherwise a new one minted with the stored key. The key is opened only here
/// (K-E1 Section 6) and dropped as soon as the token is minted.
async fn token(
    scheduling: &Scheduling,
    claim: &Claim,
    declaration: &Declaration,
) -> Result<Arc<Secret>, Failure> {
    let account = &claim.account;
    let now = scheduling.clock.now();
    if let Some(token) = scheduling.transport.tokens().fresh(account.id, now) {
        return Ok(token);
    }
    // Taken before the key is read: a removal that forgets the token after this point makes
    // the keep below store nothing.
    let ticket = scheduling.transport.tokens().ticket();
    let credential = account.credential_id.ok_or(Failure::new(
        ErrorKind::Auth,
        "No key is stored. Connect the account again.",
    ))?;
    let secret = load_source_secret(
        &scheduling.pool,
        &scheduling.vault,
        (credential, account.id),
    )
    .await
    .map_err(|error| match error {
        credentials::Error::Database(_) => super::Failure::store(),
        _ => Failure::new(
            ErrorKind::Auth,
            "The stored key cannot be opened. Connect the account again.",
        ),
    })?;
    let key = stored_key::from_secret(&secret, &account.instance_key).map_err(|_| {
        Failure::new(
            ErrorKind::Auth,
            "The stored key does not belong to this account. Connect the account again.",
        )
    })?;
    drop(secret);
    let minted = mint(
        &scheduling.transport,
        declaration,
        &key,
        (now, &SystemRandom::new()),
    )
    .await
    .map_err(|failure| match failure {
        MintFailure::Refused => REFUSED_KEY,
        MintFailure::RateLimited(until) => rate_limited(until),
        MintFailure::Unreachable => {
            Failure::new(ErrorKind::Network, "The source could not be reached.")
        }
        MintFailure::Unusable | MintFailure::NotAppKey | MintFailure::Unsigned => {
            Failure::new(ErrorKind::Provider, "The source did not give a token.")
        }
    })?;
    Ok(scheduling
        .transport
        .tokens()
        .keep(account.id, minted, ticket))
}
