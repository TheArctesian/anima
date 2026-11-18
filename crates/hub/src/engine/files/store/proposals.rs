//! The SQL of `files.proposal`.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, types::Json};

use super::unreadable;
use crate::{
    engine::{
        files::proposals::{Field, Origin, Proposal, ProposalState},
        records::Reference,
    },
    ids::{AttachmentId, ProposalId},
};

const COLUMNS: &str =
    "id, attachment_id, owner, form, origin, fields, state, created_at, decided_at";

#[derive(sqlx::FromRow)]
struct ProposalRow {
    id: ProposalId,
    attachment_id: Option<AttachmentId>,
    owner: String,
    form: String,
    origin: String,
    fields: Json<Vec<Field>>,
    state: String,
    created_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
}

impl TryFrom<ProposalRow> for Proposal {
    type Error = sqlx::Error;

    fn try_from(row: ProposalRow) -> Result<Self, Self::Error> {
        let origin = match row.origin.as_str() {
            "client" => Origin::Client,
            "hub" => Origin::Hub,
            _ => return Err(unreadable("origin")),
        };
        let state = match row.state.as_str() {
            "proposed" => ProposalState::Proposed,
            "confirmed" => ProposalState::Confirmed,
            "rejected" => ProposalState::Rejected,
            _ => return Err(unreadable("state")),
        };
        Ok(Self {
            id: row.id,
            attachment: row.attachment_id,
            owner: Reference::parse(&row.owner).map_err(|_| unreadable("owner"))?,
            form: row.form,
            origin,
            fields: row.fields.0,
            state,
            created_at: row.created_at,
            decided_at: row.decided_at,
        })
    }
}

pub(in crate::engine::files) async fn insert_proposal(
    conn: &mut PgConnection,
    proposal: &Proposal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO files.proposal (id, attachment_id, owner, form, origin, fields, state, \
         created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(proposal.id)
    .bind(proposal.attachment)
    .bind(proposal.owner.as_str())
    .bind(&proposal.form)
    .bind(proposal.origin.as_str())
    .bind(Json(&proposal.fields))
    .bind(proposal.state.as_str())
    .bind(proposal.created_at)
    .execute(conn)
    .await
    .map(drop)
}

/// Decides a proposal that is still proposed: sets its state and, for a confirm, its fields.
/// `None` when no proposed proposal has this ID.
pub(in crate::engine::files) async fn decide(
    conn: &mut PgConnection,
    id: ProposalId,
    (state, fields): (ProposalState, Option<&[Field]>),
    (at, by): (DateTime<Utc>, &str),
) -> Result<Option<Proposal>, sqlx::Error> {
    let row: Option<ProposalRow> = sqlx::query_as(&format!(
        "UPDATE files.proposal SET state = $2, fields = COALESCE($3, fields), decided_at = $4, \
         decided_by = $5 WHERE id = $1 AND state = 'proposed' RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(state.as_str())
    .bind(fields.map(Json))
    .bind(at)
    .bind(by)
    .fetch_optional(conn)
    .await?;
    row.map(Proposal::try_from).transpose()
}

pub(in crate::engine::files) async fn proposal(
    db: impl PgExecutor<'_>,
    id: ProposalId,
) -> Result<Option<Proposal>, sqlx::Error> {
    let row: Option<ProposalRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM files.proposal WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Proposal::try_from).transpose()
}

/// Up to `limit` proposals, newest first, of `owner` and in `state` when they are given.
pub(in crate::engine::files) async fn proposals(
    db: impl PgExecutor<'_>,
    owner: Option<&Reference>,
    state: Option<ProposalState>,
    limit: i64,
) -> Result<Vec<Proposal>, sqlx::Error> {
    let rows: Vec<ProposalRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM files.proposal WHERE ($1::text IS NULL OR owner = $1) \
         AND ($2::text IS NULL OR state = $2) ORDER BY created_at DESC, id DESC LIMIT $3"
    ))
    .bind(owner.map(Reference::as_str))
    .bind(state.map(ProposalState::as_str))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Proposal::try_from).collect()
}
