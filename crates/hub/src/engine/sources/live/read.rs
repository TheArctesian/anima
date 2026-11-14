//! One live read: the cache, the budget, the pace, the token and the send, in that order
//! (K-E1 Section 13.1).

use std::sync::Arc;

use source_exchange::Request;

use super::{
    Keep, LiveFailure, Reply,
    budget::limited_until,
    failed, reachable,
    replies::{Held, Key},
    token::token,
};
use crate::{
    engine::sources::{
        Scheduling,
        acquisition::transport::{Content, Fault},
    },
    ids::SourceAccountId,
};

/// Reads `request` for `account`: the held answer while `keep` says it is fresh, otherwise a
/// new `GET` with the account's installation token. A 2xx answer is held as `keep` says; any
/// other answer the source gave is returned as it is and not held. When the read fails for a
/// passing reason (a rate limit or the reserve, an unreachable source, a 5xx, a full queue), an
/// answer held for at most an hour comes back stale instead.
///
/// # Errors
/// [`LiveFailure`]: `NotFound` or `Paused` for the account, `KeyRefused` on a 401 (the token is
/// dropped and nothing is written), `Refused` for a path the hub does not send, and the
/// passing failures when nothing is held.
pub async fn ask(
    s: &Scheduling,
    account: SourceAccountId,
    request: &Request,
    keep: Keep,
) -> Result<Reply, LiveFailure> {
    let key = Key::of(account, request);
    let live = s.transport.live();
    let now = s.clock.now();
    if let Some(held) = live.replies(|r| r.fresh(&key, keep, now)) {
        return Ok(reply(held, None));
    }
    match send(s, account, request).await {
        Ok(answer) => Ok(settle(s, (key, keep), answer)),
        Err(failure) if failure.passes() => {
            let now = s.clock.now();
            live.replies(|r| r.stale(&key, keep, now))
                .map(|held| reply(held, Some(failure)))
                .ok_or(failure)
        }
        Err(failure) => Err(failure),
    }
}

/// An answer the source gave, usable by the provider: its status and its body.
struct Answered {
    status: u16,
    body: Option<Arc<[u8]>>,
}

/// Sends the read when the account, the budget and the pace allow it.
async fn send(
    s: &Scheduling,
    id: SourceAccountId,
    request: &Request,
) -> Result<Answered, LiveFailure> {
    let (account, declaration) = reachable(s, id).await?;
    let url = s
        .transport
        .url(declaration.api_base, request)
        .map_err(|_| LiveFailure::Refused)?;
    let live = s.transport.live();
    let now = s.clock.now();
    live.budget(|b| b.read(id, now))
        .map_err(|until| LiveFailure::Limited { until })?;
    let _turn = live.pace.read(id).await.map_err(|_| LiveFailure::Busy)?;
    let token = token(s, &account, &declaration)
        .await
        .map_err(|failure| failed(failure, s.clock.now()))?;
    let answer = s
        .transport
        .get(request, url, &token)
        .await
        .map_err(|fault| match fault {
            Fault::Refused => LiveFailure::Refused,
            Fault::Network => LiveFailure::Unreachable,
        })?;
    let now = s.clock.now();
    live.budget(|b| b.note(id, answer.limits, now));
    if answer.status == 401 {
        s.transport.tokens().forget(id);
        return Err(LiveFailure::KeyRefused);
    }
    if let Some(until) = limited_until(answer.status, answer.limits, now) {
        live.budget(|b| b.limited(id, until));
        return Err(LiveFailure::Limited { until });
    }
    if answer.status >= 500 {
        return Err(LiveFailure::Unusable);
    }
    Ok(Answered {
        status: answer.status,
        body: match &answer.content {
            Content::Body(bytes) => Some(Arc::from(bytes.as_slice())),
            Content::TooLarge => None,
        },
    })
}

/// Holds a 2xx answer as `keep` says and drops the older one otherwise; returns it as a reply.
fn settle(s: &Scheduling, (key, keep): (Key, Keep), answer: Answered) -> Reply {
    let at = s.clock.now();
    let live = s.transport.live();
    match (&answer.body, answer.status) {
        (Some(body), 200..=299) => {
            let held = Held {
                status: answer.status,
                body: body.clone(),
                at,
            };
            live.replies(|r| r.keep(key, keep, held));
        }
        _ => live.replies(|r| r.discard(&key)),
    }
    Reply {
        status: answer.status,
        body: answer.body,
        at,
        problem: None,
    }
}

fn reply(held: Held, problem: Option<LiveFailure>) -> Reply {
    Reply {
        status: held.status,
        body: Some(held.body),
        at: held.at,
        problem,
    }
}
