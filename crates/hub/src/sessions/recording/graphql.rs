//! The recordings GraphQL part (K1; RA-041, D-13, D-15, API-003): the catalogue
//! (`recordings`), a playback ticket (`openRecording`) and the delete (`deleteRecording`),
//! merged into the root `Query` and `Mutation`. The access backstop keeps API keys from
//! `openRecording` and `deleteRecording` before any resolver runs (D-23, R-100).

pub(super) mod connection;

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};
use sqlx::PgPool;

use self::connection::{DEFAULT_FIRST, RecordingConnection, recordings_args};
use super::{Error, deleting::delete_recording, store, viewing::open_recording};
use crate::{
    access::Authenticated,
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, schema_dependency},
    clock::Clock,
    events::{ClientAddr, Operator, Trail},
    ids::RecordingId,
    sessions::{Actor, Live, Recording, Ticket},
};

const UNAVAILABLE_MESSAGE: &str = "The hub cannot open a recording now. Try again in a moment.";

/// Who acts, from where and when, as the schema data of a signed-in request gives it.
fn actor(ctx: &Context<'_>) -> Result<Actor> {
    Ok(Actor {
        operator: schema_dependency::<Operator>(ctx)?.clone(),
        client: *schema_dependency::<ClientAddr>(ctx)?,
        at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
    })
}

/// The recording `id` names, or a `VALIDATION` error on `id`.
fn recording_id(id: &ID) -> Result<RecordingId> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That recording id is not valid.")]))
}

/// The GraphQL error with its `extensions.code` (K7, API-010). No text holds a path, a key or
/// content (LOG-007); a dependency failure shows no detail.
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(id) => coded(format!("No such recording {id}."), ErrorCode::NotFound),
        Error::ShuttingDown | Error::Busy => {
            coded(UNAVAILABLE_MESSAGE.to_owned(), ErrorCode::Unavailable)
        }
        other => internal(other),
    }
}

#[derive(Default)]
pub struct Query;

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "RecordingsQuery")]
impl Query {
    /// Recordings newest first, of every device or of one (RA-041). An ended recording past
    /// its expiry is not listed (D-25).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.recordings"
    )]
    async fn recordings(
        &self,
        ctx: &Context<'_>,
        device_id: Option<ID>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<RecordingConnection> {
        let args = recordings_args(device_id.as_ref(), first, after.as_deref())
            .map_err(|fields| invalid(&fields))?;
        let db = schema_dependency::<PgPool>(ctx)?;
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let fetched = store::page(db, (args.device, args.after), args.limit(), now)
            .await
            .map_err(to_graphql)?;
        Ok(RecordingConnection::of(fetched, args.size))
    }
}

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`.
#[Object(name = "RecordingsMutation")]
impl Mutation {
    /// A ticket for the playback tunnel (K4 §12). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn open_recording(&self, ctx: &Context<'_>, id: ID) -> Result<Ticket> {
        let id = recording_id(&id)?;
        let by = actor(ctx)?;
        open_recording(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            &by,
            id,
        )
        .await
        .map_err(to_graphql)
    }

    /// Deletes the key first, then the file (D-13), then withdraws the recording's pending
    /// playback tickets (G-7). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn delete_recording(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let recording = recording_id(&id)?;
        let playback = schema_dependency::<Live>(ctx)?.playback();
        delete_recording(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Recording>(ctx)?.root.as_deref(),
            Trail::from_context(ctx)?,
            recording,
        )
        .await
        .map_err(to_graphql)?;
        playback.withdraw_recording(recording);
        Ok(ID(recording.to_string()))
    }
}
