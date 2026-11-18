//! `Query.extractionProposals`, `Mutation.proposeExtraction`, `Mutation.confirmExtraction` and
//! `Mutation.rejectExtraction` (K-E4 Section 10; VIS-001 to VIS-004).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Context, Enum, ID, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use super::{reference, refused};
use crate::{
    api_errors::{ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool},
    engine::{
        extensions::Key,
        files::{
            proposals::{self, Field, FieldType, NewProposal, Origin, Proposal, ProposalState},
            trail,
        },
    },
    fleet::{access::Authenticated, events::Trail},
    ids::{AttachmentId, ProposalId},
};

/// K1 `ExtractedFieldType`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ExtractedFieldType")]
enum FieldTypeView {
    Text,
    Number,
    Date,
    Boolean,
    Reference,
}

/// K1 `ExtractionOrigin`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ExtractionOrigin")]
enum OriginView {
    Client,
    Hub,
}

/// K1 `ProposalState`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ProposalState")]
enum StateView {
    Proposed,
    Confirmed,
    Rejected,
}

impl From<FieldType> for FieldTypeView {
    fn from(kind: FieldType) -> Self {
        match kind {
            FieldType::Text => Self::Text,
            FieldType::Number => Self::Number,
            FieldType::Date => Self::Date,
            FieldType::Boolean => Self::Boolean,
            FieldType::Reference => Self::Reference,
        }
    }
}

impl From<FieldTypeView> for FieldType {
    fn from(kind: FieldTypeView) -> Self {
        match kind {
            FieldTypeView::Text => Self::Text,
            FieldTypeView::Number => Self::Number,
            FieldTypeView::Date => Self::Date,
            FieldTypeView::Boolean => Self::Boolean,
            FieldTypeView::Reference => Self::Reference,
        }
    }
}

impl From<ProposalState> for StateView {
    fn from(state: ProposalState) -> Self {
        match state {
            ProposalState::Proposed => Self::Proposed,
            ProposalState::Confirmed => Self::Confirmed,
            ProposalState::Rejected => Self::Rejected,
        }
    }
}

/// K1 `ExtractedField`.
#[derive(SimpleObject)]
#[graphql(name = "ExtractedField")]
struct FieldView {
    name: String,
    #[graphql(name = "type")]
    kind: FieldTypeView,
    value: String,
    /// 0 to 1; null when the extractor gave none: unknown, not certain (VIS-004).
    confidence: Option<f64>,
}

/// K1 `ExtractedFieldInput`.
#[derive(InputObject)]
#[graphql(name = "ExtractedFieldInput")]
struct FieldInput {
    name: String,
    #[graphql(name = "type")]
    kind: FieldTypeView,
    value: String,
    confidence: Option<f64>,
}

impl From<FieldInput> for Field {
    fn from(input: FieldInput) -> Self {
        Self {
            name: input.name,
            kind: input.kind.into(),
            value: input.value,
            confidence: input.confidence,
        }
    }
}

/// K1 `ExtractionProposal`.
#[derive(SimpleObject)]
#[graphql(name = "ExtractionProposal")]
struct ProposalView {
    id: ID,
    attachment_id: Option<ID>,
    owner: String,
    form: String,
    origin: OriginView,
    fields: Vec<FieldView>,
    state: StateView,
    created_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
}

impl From<Proposal> for ProposalView {
    fn from(proposal: Proposal) -> Self {
        Self {
            id: ID(proposal.id.to_string()),
            attachment_id: proposal.attachment.map(|id| ID(id.to_string())),
            owner: proposal.owner.to_string(),
            form: proposal.form,
            origin: match proposal.origin {
                Origin::Client => OriginView::Client,
                Origin::Hub => OriginView::Hub,
            },
            fields: (proposal.fields.into_iter())
                .map(|field| FieldView {
                    name: field.name,
                    kind: field.kind.into(),
                    value: field.value,
                    confidence: field.confidence,
                })
                .collect(),
            state: proposal.state.into(),
            created_at: proposal.created_at,
            decided_at: proposal.decided_at,
        }
    }
}

/// K1 `ProposeExtractionInput`.
#[derive(InputObject)]
#[graphql(name = "ProposeExtractionInput")]
struct ProposeInput {
    owner: String,
    form: String,
    /// The extension that asked for the form; its data class decides where the extraction may
    /// run (VIS-002, VLT-001).
    extension: String,
    origin: OriginView,
    attachment_id: Option<ID>,
    fields: Vec<FieldInput>,
}

fn new_proposal(input: ProposeInput) -> Result<NewProposal> {
    let owner = reference("owner", &input.owner)?;
    let extension = Key::parse(&input.extension).ok_or_else(|| {
        invalid(&[FieldError::new(
            "extension",
            "Give the key of an extension.",
        )])
    })?;
    let attachment = input
        .attachment_id
        .map(|id| id.parse::<AttachmentId>())
        .transpose()
        .map_err(|_| {
            invalid(&[FieldError::new(
                "attachmentId",
                "Give the ID of an attachment.",
            )])
        })?;
    Ok(NewProposal {
        owner,
        form: input.form,
        extension,
        origin: match input.origin {
            OriginView::Client => Origin::Client,
            OriginView::Hub => Origin::Hub,
        },
        attachment,
        fields: input.fields.into_iter().map(Into::into).collect(),
    })
}

fn proposal_id(id: &ID) -> Result<ProposalId> {
    id.parse()
        .map_err(|_| coded("No proposal has this ID.", ErrorCode::NotFound))
}

#[derive(Default)]
pub struct ProposalQuery;

#[Object(name = "ExtractionQuery")]
impl ProposalQuery {
    /// The proposals of `owner` (or of every record) in `state` (or any), newest first, at
    /// most 200.
    #[graphql(guard = "Authenticated")]
    async fn extraction_proposals(
        &self,
        ctx: &Context<'_>,
        owner: Option<String>,
        state: Option<StateView>,
    ) -> Result<Vec<ProposalView>> {
        let owner = owner.map(|owner| reference("owner", &owner)).transpose()?;
        let state = state.map(|state| match state {
            StateView::Proposed => ProposalState::Proposed,
            StateView::Confirmed => ProposalState::Confirmed,
            StateView::Rejected => ProposalState::Rejected,
        });
        let found = proposals::list(pool(ctx)?, owner.as_ref(), state)
            .await
            .map_err(refused)?;
        Ok(found.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct ProposalMutation;

#[Object(name = "ExtractionMutation")]
impl ProposalMutation {
    /// Stores the result of one extraction as a proposal (VIS-001).
    #[graphql(guard = "Authenticated")]
    async fn propose_extraction(
        &self,
        ctx: &Context<'_>,
        input: ProposeInput,
    ) -> Result<ProposalView> {
        let new = new_proposal(input)?;
        let trail_of_change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let proposal = proposals::propose(&mut tx, new, trail_of_change.now)
            .await
            .map_err(refused)?;
        written(trail_of_change, &mut tx, &proposal).await?;
        commit(tx).await?;
        Ok(proposal.into())
    }

    /// Confirms a proposal with the fields as the operator left them (VIS-001).
    #[graphql(guard = "Authenticated")]
    async fn confirm_extraction(
        &self,
        ctx: &Context<'_>,
        id: ID,
        fields: Vec<FieldInput>,
    ) -> Result<ProposalView> {
        let id = proposal_id(&id)?;
        let fields = fields.into_iter().map(Into::into).collect();
        let trail_of_change = Trail::from_context(ctx)?;
        let (now, by) = (trail_of_change.now, trail_of_change.operator().to_string());
        let mut tx = begin(ctx).await?;
        let proposal = proposals::confirm(&mut tx, id, fields, (now, &by))
            .await
            .map_err(refused)?;
        written(trail_of_change, &mut tx, &proposal).await?;
        commit(tx).await?;
        Ok(proposal.into())
    }

    /// Rejects a proposal: no record is made from it.
    #[graphql(guard = "Authenticated")]
    async fn reject_extraction(&self, ctx: &Context<'_>, id: ID) -> Result<ProposalView> {
        let id = proposal_id(&id)?;
        let trail_of_change = Trail::from_context(ctx)?;
        let (now, by) = (trail_of_change.now, trail_of_change.operator().to_string());
        let mut tx = begin(ctx).await?;
        let proposal = proposals::reject(&mut tx, id, (now, &by))
            .await
            .map_err(refused)?;
        written(trail_of_change, &mut tx, &proposal).await?;
        commit(tx).await?;
        Ok(proposal.into())
    }
}

async fn written(
    trail_of_change: Trail<'_>,
    tx: &mut sqlx::PgConnection,
    proposal: &Proposal,
) -> Result<()> {
    trail_of_change
        .write(tx, trail::proposal(proposal))
        .await
        .map_err(internal)
}
