//! The events GraphQL part: the `events` and `audit` lists and the `eventAdded` subscription
//! (K1; LOG-004, LOG-005, LOG-010, API-003, API-006).

mod connection;

use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions as _, ID, InputObject, Object, Result,
    futures_util::{Stream, stream},
    value,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use self::connection::{AuditConnection, EventConnection, EventNode};
use super::{
    Bus, Signal,
    model::{EventFilter, EventSummary, Severity, Source},
    page::{MAX_PAGE_SIZE, Position, decode_cursor, fetch_limit, page_size, split_page},
    retention::Log,
    store,
};
use crate::access::Authenticated;

/// K1's default for `first` on both lists.
const DEFAULT_FIRST: i32 = 100;

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

#[derive(Default)]
pub struct Query;

#[derive(Default)]
pub struct Subscription;

/// K1 `EventFilter`.
#[derive(InputObject, Default)]
#[graphql(name = "EventFilter")]
struct EventFilterInput {
    /// Only these severities; an empty list matches nothing.
    severity: Option<Vec<Severity>>,
    /// Only these sources; an empty list matches nothing.
    source: Option<Vec<Source>>,
    device_id: Option<ID>,
    /// Only events at or after this instant.
    since: Option<DateTime<Utc>>,
}

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "EventsQuery")]
impl Query {
    /// The event log, newest first (LOG-004).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.events"
    )]
    async fn events(
        &self,
        ctx: &Context<'_>,
        filter: Option<EventFilterInput>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<EventConnection> {
        let paged = paging(Log::Events, first, after.as_deref());
        let filter = event_filter(filter.unwrap_or_default());
        let ((size, after), filter) = match (paged, filter) {
            (Ok(paged), Ok(filter)) => (paged, filter),
            (paged, filter) => {
                return Err(invalid(paged.err().into_iter().chain(filter.err())));
            }
        };
        let rows = store::list_events(dependency(ctx)?, &filter, after, fetch_limit(size))
            .await
            .map_err(internal)?;
        Ok(split_page(rows, size).into())
    }

    /// The audit log, newest first (LOG-005).
    #[graphql(guard = "Authenticated")]
    async fn audit(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<AuditConnection> {
        let (size, after) =
            paging(Log::Audit, first, after.as_deref()).map_err(|bad| invalid([bad]))?;
        let rows = store::list_audit(dependency::<PgPool>(ctx)?, after, fetch_limit(size))
            .await
            .map_err(internal)?;
        Ok(split_page(rows, size).into())
    }
}

#[async_graphql::Subscription(name = "EventsSubscription")]
impl Subscription {
    /// Each new event at or above `minSeverity`, as it is recorded (LOG-010). A subscriber that
    /// falls far behind skips the oldest events; the `events` list is the record.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "async-graphql requires an async subscription resolver"
    )]
    async fn event_added(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(Severity::Info)")] min_severity: Option<Severity>,
    ) -> Result<impl Stream<Item = EventNode> + use<>> {
        let receiver = dependency::<Bus>(ctx)?.subscribe();
        let min = min_severity.unwrap_or(Severity::Info);
        Ok(stream::unfold(receiver, move |receiver| {
            next_event(receiver, min)
        }))
    }
}

/// Waits for the next event at or above `min`; `None` when the bus is gone.
async fn next_event(
    mut receiver: Receiver<Signal>,
    min: Severity,
) -> Option<(EventNode, Receiver<Signal>)> {
    loop {
        match receiver.recv().await {
            Ok(signal) => {
                if let Some(event) = delivered(signal, min) {
                    return Some((event.into(), receiver));
                }
            }
            // The skipped signals are a live view's loss only; the stored list keeps them.
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return None,
        }
    }
}

/// The event a subscriber at `min` receives for `signal`, if any.
fn delivered(signal: Signal, min: Severity) -> Option<EventSummary> {
    match signal {
        Signal::Event(event) if event.severity >= min => Some(event),
        _ => None,
    }
}

/// A field the operator must fix, and why.
type Bad = (&'static str, &'static str);

/// Reads `first` (K1 default when omitted) and `after` for `log`; the first bad one.
fn paging(
    log: Log,
    first: Option<i32>,
    after: Option<&str>,
) -> Result<(usize, Option<Position>), Bad> {
    let size = page_size(first.unwrap_or(DEFAULT_FIRST)).ok_or(("first", PAGE_SIZE_MESSAGE))?;
    let after = after
        .map(|cursor| decode_cursor(log, cursor).ok_or(("after", CURSOR_MESSAGE)))
        .transpose()?;
    Ok((size, after))
}

const PAGE_SIZE_MESSAGE: &str = "Ask for between 1 and 200 entries per page.";
const CURSOR_MESSAGE: &str = "That page cursor is not valid. Start again from the first page.";
const DEVICE_MESSAGE: &str = "That device id is not valid.";

const _: () = assert!(MAX_PAGE_SIZE == 200, "PAGE_SIZE_MESSAGE names the maximum");

/// The event filter the input asks for; a bad device id is refused.
fn event_filter(input: EventFilterInput) -> Result<EventFilter, Bad> {
    let device = input
        .device_id
        .map(|id| id.parse().map_err(|_| ("filter.deviceId", DEVICE_MESSAGE)))
        .transpose()?;
    Ok(EventFilter {
        severities: input.severity,
        sources: input.source,
        device,
        since: input.since,
    })
}

/// A `VALIDATION` error that lists every field the operator must fix (K7).
fn invalid(fields: impl IntoIterator<Item = Bad>) -> GraphQlError {
    let listed: Vec<_> = fields
        .into_iter()
        .map(|(field, message)| value!({ "field": field, "message": message }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", "VALIDATION");
        extensions.set("fields", async_graphql::Value::List(listed));
    })
}

/// The generic `INTERNAL` error (API-010). The cause stays as the error's source, which is
/// never serialised.
fn internal(cause: impl std::error::Error + Send + Sync + 'static) -> GraphQlError {
    let mut error = GraphQlError::new_with_source(cause);
    INTERNAL_MESSAGE.clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", "INTERNAL"))
}

/// A schema datum a resolver needs is not registered: a wiring mistake, never the operator's.
#[derive(Debug, thiserror::Error)]
#[error("schema dependency not registered")]
struct Unregistered;

fn dependency<'a, T: Send + Sync + 'static>(ctx: &Context<'a>) -> Result<&'a T> {
    ctx.data_opt::<T>().ok_or_else(|| internal(Unregistered))
}

#[cfg(test)]
mod tests;
