//! One named change: the account, the pace, the token, the send and the provider's reading of
//! the reply (K-E1 Section 13.2). The hub never sends a change twice.

use source_exchange::{Body, Change, ChangeReply, Settled};

use super::{LiveFailure, Reserve, budget::limited_until, failed, reachable, token::token};
use crate::{
    engine::sources::{
        Scheduling,
        acquisition::transport::{Content, Delivery, Fault},
    },
    ids::SourceAccountId,
};

/// Makes `change` with `account`'s source: the provider of the account's declaration renders
/// it, the transport sends it once, and the provider's `settle` reads every reply that came
/// back, a 401 or a rate limit included. A change with [`Reserve::Spendable`] is not held back
/// by the live reads' reserve; one with [`Reserve::Kept`] stops at it, as a read does. It waits
/// for the account's previous change and a second after it. When the request may have reached
/// the source but no whole answer came back the outcome is [`Settled::Unknown`], and the hub
/// does not send it again. A 401 also drops the token.
///
/// # Errors
/// [`LiveFailure`] when nothing was sent: `NotFound` or `Paused` for the account, `NoChanges`
/// for a source that makes none, `Limited` at the reserve when it is kept, `Refused` for a
/// rendered path or body the hub does not send, `Unreachable` when no connection was made,
/// `Busy` when the account's queue is full, and the token's failures.
pub async fn change(
    s: &Scheduling,
    account: SourceAccountId,
    change: &Change,
    reserve: Reserve,
) -> Result<Settled, LiveFailure> {
    let (stored, declaration) = reachable(s, account).await?;
    let changes = declaration.changes.ok_or(LiveFailure::NoChanges)?;
    let live = s.transport.live();
    if reserve == Reserve::Kept {
        live.budget(|b| b.read(account, s.clock.now()))
            .map_err(|until| LiveFailure::Limited { until })?;
    }
    let turn = live
        .pace
        .change(account, s.clock.clone())
        .await
        .map_err(|_| LiveFailure::Busy)?;
    let token = token(s, &stored, &declaration)
        .await
        .map_err(|failure| failed(failure, s.clock.now()))?;
    let delivery = s
        .transport
        .change(&declaration, change, &token)
        .await
        .map_err(|fault| match fault {
            Fault::Refused => LiveFailure::Refused,
            Fault::Network => LiveFailure::Unreachable,
        })?;
    drop(turn);
    let Delivery::Answered(answer) = delivery else {
        return Ok(Settled::Unknown);
    };
    let at = s.clock.now();
    live.budget(|b| b.note(account, answer.limits, at));
    if let Some(until) = limited_until(answer.status, answer.limits, at) {
        live.budget(|b| b.limited(account, until));
    }
    if answer.status == 401 {
        s.transport.tokens().forget(account);
    }
    let reply = ChangeReply {
        status: answer.status,
        body: match &answer.content {
            Content::Body(bytes) => Body::Read(bytes),
            Content::TooLarge => Body::TooLarge,
        },
        limit: answer.limit(),
        at,
    };
    Ok((changes.settle)(change, &reply))
}
