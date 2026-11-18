//! The files GraphQL part (K1; K-E4 Section 13): `Query.attachments`,
//! `Mutation.deleteAttachment`, `Mutation.createArchive` and `Mutation.deleteArchive`; the
//! extraction proposals are in `proposals`. The bytes of a file go through `/files/v1`.
//!
//! Each change writes its rows, one audit entry and one event in one transaction.

mod proposals;

use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions as _, ID, Object, Result, SimpleObject, Value,
};
use chrono::{DateTime, Utc};

pub use proposals::{ProposalMutation, ProposalQuery};

use super::{
    Bad, Error, Store, attachments, create_archive, delete_archive,
    model::{Archive, Attachment},
    remove_attachment, trail,
};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    engine::records::Reference,
    fleet::{access::Authenticated, events::Trail},
    ids::{ArchiveId, AttachmentId},
};

const BAD_REFERENCE: &str = "Give a record as kind:id, for example book:1234.";

/// K1 `Attachment`.
#[derive(SimpleObject)]
#[graphql(name = "Attachment")]
struct AttachmentView {
    id: ID,
    owner: String,
    name: String,
    media_type: String,
    hash: String,
    key: String,
    size: i64,
    archive_id: Option<ID>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    created_at: DateTime<Utc>,
}

impl From<Attachment> for AttachmentView {
    fn from(attachment: Attachment) -> Self {
        Self {
            id: ID(attachment.id.to_string()),
            owner: attachment.owner.to_string(),
            name: attachment.name,
            media_type: attachment.media_type,
            hash: attachment.hash,
            key: attachment.key_name,
            size: i64::try_from(attachment.size).unwrap_or(i64::MAX),
            archive_id: attachment.archive.map(|id| ID(id.to_string())),
            latitude: attachment.position.map(|p| p.latitude),
            longitude: attachment.position.map(|p| p.longitude),
            created_at: attachment.created_at,
        }
    }
}

/// K1 `Archive`.
#[derive(SimpleObject)]
#[graphql(name = "Archive")]
struct ArchiveView {
    id: ID,
    owner: String,
    name: String,
    created_at: DateTime<Utc>,
}

impl From<Archive> for ArchiveView {
    fn from(archive: Archive) -> Self {
        Self {
            id: ID(archive.id.to_string()),
            owner: archive.owner.to_string(),
            name: archive.name,
            created_at: archive.created_at,
        }
    }
}

/// `VALIDATION` with each field to fix, named as the client names it.
fn invalid_fields(bad: &[Bad]) -> GraphQlError {
    let listed: Vec<Value> = bad
        .iter()
        .map(|bad| async_graphql::value!({ "field": bad.field.clone(), "message": bad.message }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", ErrorCode::Validation.as_str());
        extensions.set("fields", Value::List(listed));
    })
}

/// The K7 code of each refusal; a fault of the hub is `INTERNAL` with no detail (API-010).
pub(super) fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(bad) => invalid_fields(&bad),
        Error::NotFound(what) => coded(format!("No {what} has this ID."), ErrorCode::NotFound),
        Error::Decided => coded(
            "The proposal is decided and does not change.",
            ErrorCode::Conflict,
        ),
        Error::Unavailable => coded(
            "This hub has no file store: set ANIMA_FILES_DIR and ANIMA_DATA_KEY_DIR.",
            ErrorCode::Unavailable,
        ),
        Error::Derived(name) => coded(
            format!("The {name} records of the archive cannot be deleted now."),
            ErrorCode::Unavailable,
        ),
        other => internal(other),
    }
}

/// The record that `text` names, or `VALIDATION` on `field`.
pub(super) fn reference(field: &'static str, text: &str) -> Result<Reference> {
    Reference::parse(text).map_err(|_| invalid(&[FieldError::new(field, BAD_REFERENCE)]))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "FilesQuery")]
impl Query {
    /// The files of `owner` (`kind:id`), newest first, at most 200.
    #[graphql(guard = "Authenticated")]
    async fn attachments(&self, ctx: &Context<'_>, owner: String) -> Result<Vec<AttachmentView>> {
        let owner = reference("owner", &owner)?;
        let found = attachments(pool(ctx)?, &owner).await.map_err(refused)?;
        Ok(found.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "FilesMutation")]
impl Mutation {
    /// Removes one file and returns its ID; its piece goes when no other file has the same
    /// bytes and key. `NOT_FOUND` when no file has this ID.
    #[graphql(guard = "Authenticated")]
    async fn delete_attachment(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let missing = || coded("No attachment has this ID.", ErrorCode::NotFound);
        let attachment_id: AttachmentId = id.parse().map_err(|_| missing())?;
        let store = schema_dependency::<Store>(ctx)?;
        let mut tx = begin(ctx).await?;
        let removed = remove_attachment(&mut tx, store, attachment_id)
            .await
            .map_err(refused)?
            .ok_or_else(missing)?;
        for entry in trail::removed(&removed) {
            let each = Trail::from_context(ctx)?;
            each.write(&mut tx, entry).await.map_err(internal)?;
        }
        commit(tx).await?;
        Ok(id)
    }

    /// Makes an archive, the unit of deletion of one import (FIL-004).
    #[graphql(guard = "Authenticated")]
    async fn create_archive(
        &self,
        ctx: &Context<'_>,
        owner: String,
        name: String,
    ) -> Result<ArchiveView> {
        let owner = reference("owner", &owner)?;
        let trail_of_change = Trail::from_context(ctx)?;
        let (now, by) = (trail_of_change.now, trail_of_change.operator().to_string());
        let mut tx = begin(ctx).await?;
        let archive = create_archive(&mut tx, (owner, &name), (now, &by))
            .await
            .map_err(refused)?;
        trail_of_change
            .write(&mut tx, trail::archive_created(&archive))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(archive.into())
    }

    /// Deletes an archive with each of its files and each record that came from it, then the
    /// pieces that no file uses (FIL-004). Returns its ID; `NOT_FOUND` when there is none.
    #[graphql(guard = "Authenticated")]
    async fn delete_archive(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let missing = || coded("No archive has this ID.", ErrorCode::NotFound);
        let archive_id: ArchiveId = id.parse().map_err(|_| missing())?;
        let store = schema_dependency::<Store>(ctx)?;
        let mut tx = begin(ctx).await?;
        let (archive, removed) = delete_archive(&mut tx, store, archive_id)
            .await
            .map_err(refused)?
            .ok_or_else(missing)?;
        for entry in trail::removed(&removed) {
            let each = Trail::from_context(ctx)?;
            each.write(&mut tx, entry).await.map_err(internal)?;
        }
        let entry = trail::archive_deleted(&archive, &removed);
        Trail::from_context(ctx)?
            .write(&mut tx, entry)
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }
}
