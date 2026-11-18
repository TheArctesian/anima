//! The audit entries and events of the files domain (LOG-004, LOG-005, LOG-007). Pure. An entry
//! holds the owner, the name, the size and the hash of a file: never a byte of it, never a key
//! and never the value of an extracted field. An event names the owner only.

use serde_json::{Value, json};

use super::{
    model::{Archive, Attachment, Removed},
    proposals::{Proposal, ProposalState},
};
use crate::fleet::events::{AuditAction, AuditTarget, Severity, Source, TrailEntry};

fn entry(action: AuditAction, target: AuditTarget, message: String, change: Value) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target,
        change,
    }
}

/// What the audit entry of a file holds.
pub(super) fn file_change(attachment: &Attachment) -> Value {
    json!({
        "owner": attachment.owner.as_str(),
        "name": attachment.name,
        "size": attachment.size,
        "hash": attachment.hash,
        "key": attachment.key_name,
        "archive": attachment.archive.map(|id| id.to_string()),
        "locationKept": attachment.position.is_some(),
    })
}

pub(super) fn uploaded(attachment: &Attachment) -> TrailEntry {
    entry(
        AuditAction::Upload,
        AuditTarget::Attachment(attachment.id),
        format!("A file was attached to {}.", attachment.owner),
        file_change(attachment),
    )
}

/// One entry for each attachment that a delete removed.
pub(super) fn removed(removed: &Removed) -> Vec<TrailEntry> {
    removed
        .attachments
        .iter()
        .map(|attachment| {
            let mut change = file_change(attachment);
            change["pieceRemoved"] = Value::Bool(
                removed
                    .pieces
                    .iter()
                    .any(|(key, hash)| *key == attachment.key_name && *hash == attachment.hash),
            );
            entry(
                AuditAction::Delete,
                AuditTarget::Attachment(attachment.id),
                format!("A file of {} was removed.", attachment.owner),
                change,
            )
        })
        .collect()
}

fn archive_change(archive: &Archive) -> Value {
    json!({ "owner": archive.owner.as_str(), "name": archive.name })
}

pub(super) fn archive_created(archive: &Archive) -> TrailEntry {
    entry(
        AuditAction::Create,
        AuditTarget::Archive(archive.id),
        format!("An archive was made for {}.", archive.owner),
        archive_change(archive),
    )
}

pub(super) fn archive_deleted(archive: &Archive, removed: &Removed) -> TrailEntry {
    let mut change = archive_change(archive);
    change["attachments"] = json!(removed.attachments.len());
    change["piecesRemoved"] = json!(removed.pieces.len());
    change["derived"] = removed
        .derived
        .iter()
        .map(|(name, count)| ((*name).to_owned(), json!(count)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    entry(
        AuditAction::Delete,
        AuditTarget::Archive(archive.id),
        format!(
            "An archive of {} was deleted with its records.",
            archive.owner
        ),
        change,
    )
}

/// The entry of a proposal: its form, origin, state and the names and types of its fields,
/// never their values.
pub(super) fn proposal(proposal: &Proposal) -> TrailEntry {
    let (action, verb) = match proposal.state {
        ProposalState::Proposed => (AuditAction::Create, "proposed"),
        ProposalState::Confirmed => (AuditAction::Accept, "confirmed"),
        ProposalState::Rejected => (AuditAction::Update, "rejected"),
    };
    let fields: Vec<Value> = proposal
        .fields
        .iter()
        .map(|field| json!({ "name": field.name, "type": field.kind }))
        .collect();
    entry(
        action,
        AuditTarget::ExtractionProposal(proposal.id),
        format!("An extraction for {} was {verb}.", proposal.owner),
        json!({
            "owner": proposal.owner.as_str(),
            "form": proposal.form,
            "origin": proposal.origin.as_str(),
            "state": proposal.state.as_str(),
            "attachment": proposal.attachment.map(|id| id.to_string()),
            "fields": fields,
        }),
    )
}
