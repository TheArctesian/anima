//! The installation token of an account, for a sync run and for a live read or change alike
//! (PRJ-018, K-E1 Sections 6 and 7). The key is opened only here.

use std::sync::Arc;

use ring::rand::SystemRandom;
use source_exchange::Declaration;

use crate::{
    engine::sources::{
        Scheduling,
        acquisition::{MintFailure, mint, stored_key},
        model::{Account, ErrorKind},
        run::Failure,
    },
    fleet::credentials::{self, Secret, load_source_secret},
};

pub(in crate::engine::sources) const REFUSED_KEY: Failure = Failure::new(
    ErrorKind::Auth,
    "The source refused the key. Connect the account again.",
);

pub(in crate::engine::sources) fn rate_limited(until: chrono::DateTime<chrono::Utc>) -> Failure {
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
pub(in crate::engine::sources) async fn token(
    scheduling: &Scheduling,
    account: &Account,
    declaration: &Declaration,
) -> Result<Arc<Secret>, Failure> {
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
        credentials::Error::Database(_) => Failure::store(),
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
