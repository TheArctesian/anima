//! `Subscription.reminderFirings`: each firing when it is sent, for an application that shows a
//! notification of the device (K-E6 Section 5; REM-008, REM-009).

use async_graphql::{
    Context, Result, Subscription as GraphQlSubscription,
    futures_util::stream::{self, Stream},
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::views::FiringView;
use crate::{
    api_errors::schema_dependency,
    engine::reminders::{Firing, Live},
    fleet::access::Authenticated,
};

/// The firings of `receiver` until the hub stops. A subscriber that fell behind skips the
/// oldest firings; each of them is in `Query.reminderFirings`.
fn sent(receiver: Receiver<Firing>) -> impl Stream<Item = FiringView> {
    stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(firing) => return Some((FiringView::from(firing), receiver)),
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    })
}

#[derive(Default)]
pub struct Subscription;

#[GraphQlSubscription(name = "RemindersSubscription")]
impl Subscription {
    /// Each firing when it is sent. While one subscription is open, the hub queues no reminder
    /// to a channel: the application shows it. For a firing about a private record the text is
    /// `You have a reminder.` and the subject is empty; `reminderFirings` of the query gives
    /// the firing itself.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn reminder_firings(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = FiringView>> {
        let live = schema_dependency::<Live>(ctx)?;
        Ok(sent(live.subscribe()))
    }
}
