//! `sourceAccountChanged`: each change the feed announces, as the account is then (K1).

use async_graphql::{Context, ID, Result, Subscription as SubscriptionObject};
use futures_util::{Stream, stream};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::{account_id, scheduling, view, views::AccountView};
use crate::{engine::sources::Scheduling, fleet::access::Authenticated, ids::SourceAccountId};

#[derive(Default)]
pub struct Subscription;

#[SubscriptionObject(name = "SourcesSubscription")]
impl Subscription {
    /// Each change of an account (of `id` only, when given): a sync that starts or ends, a
    /// pause, a new key. A removed account sends nothing more.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn source_account_changed(
        &self,
        ctx: &Context<'_>,
        id: Option<ID>,
    ) -> Result<impl Stream<Item = AccountView>> {
        let wanted = id.as_ref().map(account_id).transpose()?;
        let s = scheduling(ctx)?;
        let received = s.feed.subscribe();
        Ok(stream::unfold(
            (received, s),
            move |(mut received, s)| async move {
                let shown = next_change(&mut received, wanted, &s).await?;
                Some((shown, (received, s)))
            },
        ))
    }
}

/// The next change of `wanted` (of any account when `None`) that still has an account to
/// show. A subscriber that fell behind skips what it missed; `None` when the feed closes.
async fn next_change(
    received: &mut Receiver<SourceAccountId>,
    wanted: Option<SourceAccountId>,
    s: &Scheduling,
) -> Option<AccountView> {
    loop {
        match received.recv().await {
            Ok(changed) if wanted.is_none_or(|w| w == changed) => {
                if let Ok(shown) = view(s, changed).await {
                    return Some(shown);
                }
            }
            Ok(_) | Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return None,
        }
    }
}
