//! The events GraphQL part: the `events` and `audit` lists and the `eventAdded` subscription
//! (K1; LOG-004, LOG-005, LOG-010, API-003, API-006).

mod chain;
mod connection;

use async_graphql::{
    Context, ID, InputObject, Object, Result,
    futures_util::{Stream, stream},
};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use self::{
    chain::ChainCheckNode,
    connection::{AuditConnection, EventConnection, EventNode},
};
use super::{
    Bus, Signal,
    actor::ActorKind,
    chain::check_chain,
    model::{EventFilter, EventSummary, Severity, Source},
    page::{MAX_PAGE_SIZE, Position, decode_cursor, fetch_limit, page_size, split_page},
    retention::Log,
    store,
};
use crate::{
    api_errors::{FieldError, internal, invalid, schema_dependency},
    clock::Clock,
    fleet::access::Authenticated,
};

/// K1's default for `first` on both lists.
const DEFAULT_FIRST: i32 = 100;

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
    /// Only events whose message or device name holds this text, with case ignored. 1 to 200
    /// characters; blank matches every event.
    text: Option<String>,
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
        let ((size, after), filter) =
            validate(filter, first, after.as_deref()).map_err(|bad| invalid(&bad))?;
        let rows = store::list_events(schema_dependency(ctx)?, &filter, after, fetch_limit(size))
            .await
            .map_err(internal)?;
        Ok(split_page(rows, size).into())
    }

    /// The audit log, newest first (LOG-005); only the entries of `actorKind` when it is
    /// given (LOG-020).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.audit"
    )]
    async fn audit(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
        actor_kind: Option<ActorKind>,
    ) -> Result<AuditConnection> {
        let (size, after) =
            paging(Log::Audit, first, after.as_deref()).map_err(|bad| invalid(&[bad]))?;
        let db = schema_dependency::<PgPool>(ctx)?;
        let rows = store::list_audit(db, actor_kind, after, fetch_limit(size))
            .await
            .map_err(internal)?;
        Ok(split_page(rows, size).into())
    }

    /// The check of the hash chain of the audit log (LOG-027). It changes nothing.
    #[graphql(guard = "Authenticated")]
    async fn audit_chain(&self, ctx: &Context<'_>) -> Result<ChainCheckNode> {
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let check = check_chain(schema_dependency::<PgPool>(ctx)?, now)
            .await
            .map_err(internal)?;
        ChainCheckNode::try_from(check).map_err(internal)
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
        let receiver = schema_dependency::<Bus>(ctx)?.subscribe();
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

/// A page size, and the position the page starts after.
type Paging = (usize, Option<Position>);

/// Reads the arguments of `Query.events`; every bad field at once.
fn validate(
    filter: Option<EventFilterInput>,
    first: Option<i32>,
    after: Option<&str>,
) -> Result<(Paging, EventFilter), Vec<FieldError>> {
    match (
        paging(Log::Events, first, after),
        event_filter(filter.unwrap_or_default()),
    ) {
        (Ok(paged), Ok(filter)) => Ok((paged, filter)),
        (paged, filter) => Err(paged.err().into_iter().chain(filter.err()).collect()),
    }
}

/// Reads `first` (K1 default when omitted) and `after` for `log`; the first bad one.
fn paging(log: Log, first: Option<i32>, after: Option<&str>) -> Result<Paging, FieldError> {
    let size = page_size(first.unwrap_or(DEFAULT_FIRST))
        .ok_or_else(|| FieldError::new("first", PAGE_SIZE_MESSAGE))?;
    let after = after
        .map(|cursor| {
            decode_cursor(log, cursor).ok_or_else(|| FieldError::new("after", CURSOR_MESSAGE))
        })
        .transpose()?;
    Ok((size, after))
}

const PAGE_SIZE_MESSAGE: &str = "Ask for between 1 and 200 entries per page.";
const CURSOR_MESSAGE: &str = "That page cursor is not valid. Start again from the first page.";
const DEVICE_MESSAGE: &str = "That device id is not valid.";
const TEXT_MESSAGE: &str = "Search for at most 200 characters.";
/// The longest search text, in characters.
const MAX_TEXT: usize = 200;

const _: () = assert!(MAX_PAGE_SIZE == 200, "PAGE_SIZE_MESSAGE names the maximum");

/// The event filter the input asks for; a bad device id is refused.
fn event_filter(input: EventFilterInput) -> Result<EventFilter, FieldError> {
    let device = input
        .device_id
        .map(|id| {
            id.parse()
                .map_err(|_| FieldError::new("filter.deviceId", DEVICE_MESSAGE))
        })
        .transpose()?;
    Ok(EventFilter {
        severities: input.severity,
        sources: input.source,
        device,
        since: input.since,
        text: search_text(input.text)?,
    })
}

/// The search text without the spaces around it; `None` when it is blank.
fn search_text(text: Option<String>) -> Result<Option<String>, FieldError> {
    let Some(text) = text.map(|text| text.trim().to_owned()) else {
        return Ok(None);
    };
    if text.chars().count() > MAX_TEXT {
        return Err(FieldError::new("filter.text", TEXT_MESSAGE));
    }
    Ok((!text.is_empty()).then_some(text))
}

#[cfg(test)]
mod tests;
