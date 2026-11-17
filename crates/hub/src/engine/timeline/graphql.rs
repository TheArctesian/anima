//! The timeline GraphQL part: `Query.timeline` (K1; TML-001 to TML-007, API-006).

use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use super::{Adapters, Ask, Basis, Category, Evidence, Page, cursor, read};
use crate::{
    api_errors::{FieldError, internal, invalid, pool, schema_dependency},
    fleet::access::Authenticated,
    paging::PageEnd,
};

/// K1's default for `first`.
const DEFAULT_FIRST: i32 = 100;
/// The most entries one page may hold.
const MAX_PAGE_SIZE: usize = 200;

/// K1 `Evidence`.
#[derive(SimpleObject)]
#[graphql(name = "Evidence")]
struct EvidenceNode {
    /// The record this entry is made from, as `kind:id`.
    reference: String,
    /// The key of the source that recorded it.
    source: String,
    category: Category,
    title: String,
    start: DateTime<Utc>,
    /// Null: the source gives no end time.
    end: Option<DateTime<Utc>>,
    /// When the hub received the record, when that is later than the event (TML-009). Null:
    /// the hub saw the event when it happened, as for the event log.
    received_at: Option<DateTime<Utc>>,
    basis: Basis,
    /// What the source does not say about this entry, one sentence each (TML-006).
    uncertainty: Vec<String>,
    /// The entry of a higher-ranked source for the same event (TML-005).
    superseded_by: Option<String>,
}

#[derive(SimpleObject)]
struct EvidenceEdge {
    cursor: String,
    node: EvidenceNode,
}

/// K1 `EvidenceConnection`.
#[derive(SimpleObject)]
struct EvidenceConnection {
    edges: Vec<EvidenceEdge>,
    page_info: PageEnd,
}

impl From<Evidence> for EvidenceEdge {
    fn from(entry: Evidence) -> Self {
        Self {
            cursor: cursor::encode(&entry.mark()),
            node: EvidenceNode {
                reference: entry.reference.to_string(),
                source: entry.source,
                category: entry.category,
                title: entry.title,
                start: entry.start,
                end: entry.end,
                received_at: entry.received_at,
                basis: entry.basis,
                uncertainty: entry.uncertainty,
                superseded_by: entry.superseded_by.map(|leader| leader.to_string()),
            },
        }
    }
}

impl From<Page> for EvidenceConnection {
    fn from(page: Page) -> Self {
        let edges: Vec<EvidenceEdge> = page.entries.into_iter().map(Into::into).collect();
        Self {
            page_info: PageEnd {
                has_next_page: page.has_next_page,
                end_cursor: edges.last().map(|edge| edge.cursor.clone()),
            },
            edges,
        }
    }
}

/// What the operator typed, before it is checked.
struct Typed<'a> {
    span: (DateTime<Utc>, DateTime<Utc>),
    categories: Option<Vec<Category>>,
    first: Option<i32>,
    after: Option<&'a str>,
}

/// The ask the arguments make, or each argument the operator must fix.
fn ask(typed: Typed<'_>) -> Result<Ask, Vec<FieldError>> {
    let mut bad = Vec::new();
    let (from, to) = typed.span;
    if from >= to {
        bad.push(FieldError::new("to", "The end must be after the start."));
    }
    let limit = usize::try_from(typed.first.unwrap_or(DEFAULT_FIRST))
        .ok()
        .filter(|size| (1..=MAX_PAGE_SIZE).contains(size));
    if limit.is_none() {
        bad.push(FieldError::new("first", "Give a number from 1 to 200."));
    }
    let after = typed.after.map(cursor::decode);
    if after == Some(None) {
        bad.push(FieldError::new("after", "This is not a timeline cursor."));
    }
    match limit {
        Some(limit) if bad.is_empty() => Ok(Ask {
            from,
            to,
            after: after.flatten(),
            categories: typed.categories,
            limit,
        }),
        _ => Err(bad),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "TimelineQuery")]
impl Query {
    /// What happened from `from` up to, not including, `to`, newest first. Each entry is read
    /// from its own domain's records; the timeline stores nothing (TML-001).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.timeline"
    )]
    async fn timeline(
        &self,
        ctx: &Context<'_>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        categories: Option<Vec<Category>>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<EvidenceConnection> {
        let typed = Typed {
            span: (from, to),
            categories,
            first,
            after: after.as_deref(),
        };
        let ask = ask(typed).map_err(|bad| invalid(&bad))?;
        let adapters = schema_dependency::<Adapters>(ctx)?;
        let page = read(adapters, pool(ctx)?, &ask).await.map_err(internal)?;
        Ok(page.into())
    }
}
