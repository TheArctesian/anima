//! The links GraphQL part: `Query.recordLinks`, `Mutation.linkRecords` and
//! `Mutation.unlinkRecords` (K1; RLK-001, RLK-002, RLK-004).
//!
//! Each change writes the link, its audit entry and its event in one transaction.

use async_graphql::{
    Context, Error as GraphQlError, ID, InputObject, Object, Result, SimpleObject,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use super::{
    Error, Link, NewLink, Relation, TitleSources, Unavailable, connect, disconnect, label,
    own_title, touching, trail,
};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    engine::records::Reference,
    fleet::{access::Authenticated, events::Trail},
    ids::RecordLinkId,
};

const BAD_REFERENCE: &str = "Give a record as kind:id, for example person:1234.";

/// K1 `RecordLink`.
#[derive(SimpleObject)]
#[graphql(name = "RecordLink")]
struct LinkView {
    id: ID,
    /// The record the link starts at, as `kind:id`.
    source: String,
    /// The record the link points to, as `kind:id`. It does not have to exist (RLK-002).
    target: String,
    /// What the link means, in lower snake case.
    kind: String,
    /// The text to show for the target while it has no record (RLK-002).
    label: Option<String>,
    /// The name of the source record. Null when its domain gives none.
    source_title: Option<String>,
    /// The name of the target record. Null when its domain gives none; show `label`.
    target_title: Option<String>,
    created_at: DateTime<Utc>,
}

/// The name of `record`: its own, or the one its domain gave.
fn title_of(record: &Reference, found: &HashMap<Reference, String>) -> Option<String> {
    own_title(record).or_else(|| found.get(record).cloned())
}

/// The links as the API shows them, with the titles that the domains of their records give.
async fn views(ctx: &Context<'_>, db: &PgPool, links: Vec<Link>) -> Result<Vec<LinkView>> {
    let ends: Vec<&Reference> = links
        .iter()
        .flat_map(|link| [&link.source, &link.target])
        .collect();
    let found = schema_dependency::<TitleSources>(ctx)?
        .of(db, &ends)
        .await
        .map_err(|error| unavailable(&error))?;
    Ok(links
        .into_iter()
        .map(|link| LinkView::new(link, &found))
        .collect())
}

fn unavailable(error: &Unavailable) -> GraphQlError {
    tracing::warn!(%error, "titles of linked records are not available");
    coded(
        "The names of some linked records are not available. Try again.",
        ErrorCode::Unavailable,
    )
}

impl LinkView {
    fn new(link: Link, found: &HashMap<Reference, String>) -> Self {
        Self {
            id: ID(link.id.to_string()),
            source_title: title_of(&link.source, found),
            target_title: title_of(&link.target, found),
            source: link.source.to_string(),
            target: link.target.to_string(),
            kind: link.relation.as_str().to_owned(),
            label: link.label,
            created_at: link.created_at,
        }
    }
}

/// K1 `RecordLinkInput`.
#[derive(InputObject)]
#[graphql(name = "RecordLinkInput")]
struct LinkInput {
    source: String,
    target: String,
    /// Lower snake case, 1 to 40 characters, for example `lent_to`.
    kind: String,
    /// One line of at most 200 characters.
    label: Option<String>,
}

/// The link the input asks for, or each field the operator must fix.
fn new_link(input: &LinkInput) -> Result<NewLink, Vec<FieldError>> {
    let mut bad = Vec::new();
    let source = Reference::parse(&input.source);
    let target = Reference::parse(&input.target);
    let relation = Relation::parse(&input.kind);
    let text = label(input.label.as_deref().unwrap_or_default());
    if source.is_err() {
        bad.push(FieldError::new("source", BAD_REFERENCE));
    }
    if target.is_err() {
        bad.push(FieldError::new("target", BAD_REFERENCE));
    }
    if relation.is_none() {
        bad.push(FieldError::new(
            "kind",
            "Give 1 to 40 lower-case letters, digits or underscores. Start with a letter.",
        ));
    }
    if text.is_err() {
        bad.push(FieldError::new(
            "label",
            "Give one line of at most 200 characters.",
        ));
    }
    match (source, target, relation, text) {
        (Ok(source), Ok(target), Some(relation), Ok(label)) => Ok(NewLink {
            source,
            target,
            relation,
            label,
        }),
        _ => Err(bad),
    }
}

/// `VALIDATION` for input the operator can fix; anything else is `INTERNAL` (API-010).
fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Database(_) => internal(error),
        Error::SelfLink => invalid(&[FieldError::new("target", "A record cannot link to itself.")]),
        Error::BadSubject(side) => invalid(&[FieldError::new(
            side,
            "This is not a valid repository, place, person or date key.",
        )]),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "LinksQuery")]
impl Query {
    /// The links that start or end at `record` (`kind:id`), newest first, at most 200.
    #[graphql(guard = "Authenticated")]
    async fn record_links(&self, ctx: &Context<'_>, record: String) -> Result<Vec<LinkView>> {
        let record = Reference::parse(&record)
            .map_err(|_| invalid(&[FieldError::new("record", BAD_REFERENCE)]))?;
        let links = touching(pool(ctx)?, &record).await.map_err(refused)?;
        views(ctx, pool(ctx)?, links).await
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "LinksMutation")]
impl Mutation {
    /// Links two records. Neither has to exist (RLK-002). A second call with the same source,
    /// target and kind returns the stored link; a new label replaces the old one.
    #[graphql(guard = "Authenticated")]
    async fn link_records(&self, ctx: &Context<'_>, input: LinkInput) -> Result<LinkView> {
        let new = new_link(&input).map_err(|bad| invalid(&bad))?;
        let trail_of_change = Trail::from_context(ctx)?;
        let (now, by) = (trail_of_change.now, trail_of_change.operator().to_string());
        let mut tx = begin(ctx).await?;
        let (link, outcome) = connect(&mut tx, new, (now, &by)).await.map_err(refused)?;
        if let Some(entry) = trail::connected(&link, outcome) {
            trail_of_change
                .write(&mut tx, entry)
                .await
                .map_err(internal)?;
        }
        commit(tx).await?;
        let mut shown = views(ctx, pool(ctx)?, vec![link]).await?;
        shown
            .pop()
            .ok_or_else(|| internal(Error::Database(sqlx::Error::RowNotFound)))
    }

    /// Removes one link and returns its ID. The two records are not changed. `NOT_FOUND` when
    /// there is no such link.
    #[graphql(guard = "Authenticated")]
    async fn unlink_records(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let missing = || coded("No record link has this ID.", ErrorCode::NotFound);
        let link_id: RecordLinkId = id.parse().map_err(|_| missing())?;
        let trail_of_change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let link = disconnect(&mut tx, link_id)
            .await
            .map_err(refused)?
            .ok_or_else(missing)?;
        trail_of_change
            .write(&mut tx, trail::disconnected(&link))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }
}
