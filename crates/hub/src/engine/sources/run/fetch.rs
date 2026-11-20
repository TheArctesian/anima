//! One request of a run: the address check, the token, the send, and the verdict on access
//! before any provider reads the reply (K-E1 Section 7).

use source_exchange::{Acquisition, Declaration, Request};

use super::{Claim, Failure};
use crate::engine::sources::{
    Scheduling,
    acquisition::{
        account_key,
        transport::{Answer, Fault},
        verdict::{Verdict, judge},
    },
    live::token::{REFUSED_KEY, rate_limited, token},
    model::ErrorKind,
};

/// The address of an account that the address policy of the hub refuses.
const REFUSED_ADDRESS: Failure = Failure::new(
    ErrorKind::Provider,
    "The address of this account is one that the hub does not connect to.",
);

/// A server of an account that does not answer.
const UNREACHED_ADDRESS: Failure = Failure::new(
    ErrorKind::Network,
    "The hub cannot reach the address of this account. A hub outside your network reaches only \
     an address that is open to it.",
);

const REFUSED_PATH: Failure = Failure::new(
    ErrorKind::Provider,
    "The source asked for an address the hub does not send to.",
);

/// Fetches `request` for the claimed account: refused before anything is sent when its
/// address could leave the source's API base; a 401 drops the token and is `auth`; a rate
/// limit is `rate_limit` until its end. Any other reply is the provider's to read, including one
/// larger than the hub reads: the provider decides what that means for its request.
pub(super) async fn fetch(
    scheduling: &Scheduling,
    (claim, declaration): (&Claim, &Declaration),
    request: &Request,
) -> Result<Answer, Failure> {
    if matches!(declaration.acquisition, Acquisition::AccountKey(_)) {
        return fetch_at_account(scheduling, claim, request).await;
    }
    let url = scheduling
        .transport
        .url(declaration.api_base, request)
        .map_err(|_| REFUSED_PATH)?;
    let token = token(scheduling, &claim.account, declaration).await?;
    let answer = scheduling
        .transport
        .get(request, url, &token)
        .await
        .map_err(|fault| match fault {
            Fault::Refused => REFUSED_PATH,
            Fault::Network => Failure::new(ErrorKind::Network, "The source could not be reached."),
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

/// Fetches `request` from the server of the account with its key (K-E1 Section 7b). The key
/// is opened for this request only. A 401 or a 403 with no rate limit is `auth`.
async fn fetch_at_account(
    scheduling: &Scheduling,
    claim: &Claim,
    request: &Request,
) -> Result<Answer, Failure> {
    let (base, key) = account_key::held(scheduling, &claim.account).await?;
    let answer = scheduling
        .transport
        .get_at(&base, request, &key)
        .await
        .map_err(|fault| match fault {
            Fault::Refused => REFUSED_ADDRESS,
            Fault::Network => UNREACHED_ADDRESS,
        })?;
    drop(key);
    match judge(answer.status, answer.limits, scheduling.clock.now()) {
        Verdict::Refused => Err(REFUSED_KEY),
        Verdict::RateLimited(until) => Err(rate_limited(until)),
        Verdict::Usable if answer.status == 403 => Err(REFUSED_KEY),
        Verdict::Usable => Ok(answer),
    }
}
