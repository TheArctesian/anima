//! Extraction proposals (K-E4 Section 10; VIS-001 to VIS-004). An extraction gives typed fields,
//! each with a confidence or none; nothing of it is a record until the operator confirms it,
//! and a decided proposal does not change. The engine has no extractor.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};

use super::{Bad, Error, store};
use crate::{
    engine::{
        extensions::{Key, Sensitivity},
        records::Reference,
    },
    ids::{AttachmentId, ProposalId},
};

/// The most fields in one proposal, and the longest value in characters.
const MAX_FIELDS: usize = 100;
const MAX_VALUE: usize = 2000;
/// The most proposals one list gives.
pub(super) const MAX_PROPOSALS: i64 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Date,
    Boolean,
    Reference,
}

/// One typed field (VIS-004). `value` is text in the form of its type; a `None` confidence is
/// unknown, not certain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    pub value: String,
    pub confidence: Option<f64>,
}

/// Where the extraction ran (VIS-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Client,
    Hub,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalState {
    Proposed,
    Confirmed,
    Rejected,
}

impl Origin {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Hub => "hub",
        }
    }
}

impl ProposalState {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: ProposalId,
    pub attachment: Option<AttachmentId>,
    pub owner: Reference,
    pub form: String,
    pub origin: Origin,
    /// As the extraction gave them while proposed; as the operator left them once confirmed.
    pub fields: Vec<Field>,
    pub state: ProposalState,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
}

/// A proposal to store. `extension` is the extension that asked for `form`: its data class
/// decides where the extraction may run (VIS-002, VLT-001).
#[derive(Clone, Debug)]
pub struct NewProposal {
    pub owner: Reference,
    pub form: String,
    pub extension: Key,
    pub origin: Origin,
    pub attachment: Option<AttachmentId>,
    pub fields: Vec<Field>,
}

/// Whether `text` can name a form or a field: lower snake case, 1 to 40 characters.
fn is_form(text: &str) -> bool {
    crate::engine::data_keys::is_name(text)
}

/// Whether `value` has the form of `kind`.
fn has_form(kind: FieldType, value: &str) -> bool {
    match kind {
        FieldType::Text => !value.chars().any(|c| c.is_control() && c != '\n'),
        FieldType::Number => value.parse::<f64>().is_ok_and(f64::is_finite),
        FieldType::Date => NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok(),
        FieldType::Boolean => matches!(value, "true" | "false"),
        FieldType::Reference => Reference::parse(value).is_ok(),
    }
}

/// The fields as they are stored, or each one the operator must fix: a name in lower snake case
/// once in the list, a value of at most 2 000 characters in the form of its type, a confidence
/// of 0 to 1 or none. A `None` confidence stays `None` (VIS-004).
///
/// # Errors
/// The bad fields, named `fields.<index>.<part>`.
pub fn checked_fields(fields: &[Field]) -> Result<(), Vec<Bad>> {
    let mut bad = Vec::new();
    if fields.len() > MAX_FIELDS {
        bad.push(Bad::new("fields", "Give at most 100 fields."));
    }
    for (index, field) in fields.iter().enumerate() {
        let named = |part: &str| format!("fields.{index}.{part}");
        if !is_form(&field.name) || fields[..index].iter().any(|f| f.name == field.name) {
            bad.push(Bad::new(
                named("name"),
                "Give a name of lower-case letters, digits or underscores, once.",
            ));
        }
        if field.value.chars().count() > MAX_VALUE || !has_form(field.kind, &field.value) {
            bad.push(Bad::new(
                named("value"),
                "Give a value of at most 2000 characters in the form of its type.",
            ));
        }
        if field.confidence.is_some_and(|c| !(0.0..=1.0).contains(&c)) {
            bad.push(Bad::new(
                named("confidence"),
                "Give a confidence from 0 to 1, or none.",
            ));
        }
    }
    if bad.is_empty() { Ok(()) } else { Err(bad) }
}

/// Where an extraction for an extension of `class` may come from. A Class A image never
/// reaches the hub in a form it can read (VLT-001): the hub refuses its own extraction for it,
/// and a client proposal waits for the client sealing of K-E5 Section 3 (E0-6).
fn admitted(origin: Origin, class: Sensitivity) -> Result<(), Bad> {
    match (class, origin) {
        (Sensitivity::A, Origin::Hub) => Err(Bad::new(
            "origin",
            "This form is Class A: its extraction runs on the client only.",
        )),
        (Sensitivity::A, Origin::Client) => Err(Bad::new(
            "extension",
            "A Class A proposal must be sealed on the client; the hub does not take it yet.",
        )),
        _ => Ok(()),
    }
}

/// Stores a proposal in the caller's transaction.
///
/// # Errors
/// `Invalid` for a bad form, bad fields, a refused origin or an attachment of another owner;
/// `Database`.
pub async fn propose(
    conn: &mut PgConnection,
    new: NewProposal,
    at: DateTime<Utc>,
) -> Result<Proposal, Error> {
    let mut bad = checked_fields(&new.fields).err().unwrap_or_default();
    if !is_form(&new.form) {
        bad.push(Bad::new(
            "form",
            "Give a form in lower snake case, such as garment.",
        ));
    }
    bad.extend(admitted(new.origin, new.extension.entry().data_class).err());
    if let Some(id) = new.attachment {
        let found = store::attachment(&mut *conn, id).await?;
        if found.is_none_or(|attachment| attachment.owner != new.owner) {
            bad.push(Bad::new(
                "attachmentId",
                "Give an attachment of the same owner.",
            ));
        }
    }
    if !bad.is_empty() {
        return Err(Error::Invalid(bad));
    }
    let proposal = Proposal {
        id: uuid::Uuid::now_v7().into(),
        attachment: new.attachment,
        owner: new.owner,
        form: new.form,
        origin: new.origin,
        fields: new.fields,
        state: ProposalState::Proposed,
        created_at: at,
        decided_at: None,
    };
    store::insert_proposal(conn, &proposal).await?;
    Ok(proposal)
}

/// Confirms a proposal with the fields as the operator left them (VIS-001).
///
/// # Errors
/// `Invalid` for bad fields; `NotFound`; `Decided` for a proposal that is decided; `Database`.
pub async fn confirm(
    conn: &mut PgConnection,
    id: ProposalId,
    fields: Vec<Field>,
    (at, by): (DateTime<Utc>, &str),
) -> Result<Proposal, Error> {
    checked_fields(&fields).map_err(Error::Invalid)?;
    decided(
        conn,
        id,
        (ProposalState::Confirmed, Some(&fields)),
        (at, by),
    )
    .await
}

/// Rejects a proposal; its fields stay as the extraction gave them.
///
/// # Errors
/// `NotFound`; `Decided` for a proposal that is decided; `Database`.
pub async fn reject(
    conn: &mut PgConnection,
    id: ProposalId,
    (at, by): (DateTime<Utc>, &str),
) -> Result<Proposal, Error> {
    decided(conn, id, (ProposalState::Rejected, None), (at, by)).await
}

async fn decided(
    conn: &mut PgConnection,
    id: ProposalId,
    decision: (ProposalState, Option<&[Field]>),
    (at, by): (DateTime<Utc>, &str),
) -> Result<Proposal, Error> {
    if let Some(proposal) = store::decide(conn, id, decision, (at, by)).await? {
        return Ok(proposal);
    }
    match store::proposal(&mut *conn, id).await? {
        Some(_) => Err(Error::Decided),
        None => Err(Error::NotFound("proposal")),
    }
}

/// The confirmed proposals of `owner`, newest first: what the owner extension makes its
/// records from (VIS-001).
///
/// # Errors
/// `Database` when the read fails.
pub async fn confirmed(db: &PgPool, owner: &Reference) -> Result<Vec<Proposal>, Error> {
    Ok(store::proposals(
        db,
        Some(owner),
        Some(ProposalState::Confirmed),
        MAX_PROPOSALS,
    )
    .await?)
}

/// The proposals of `owner` (or of every owner) in `state` (or any), newest first, at most 200.
///
/// # Errors
/// `Database` when the read fails.
pub async fn list(
    db: &PgPool,
    owner: Option<&Reference>,
    state: Option<ProposalState>,
) -> Result<Vec<Proposal>, Error> {
    Ok(store::proposals(db, owner, state, MAX_PROPOSALS).await?)
}
