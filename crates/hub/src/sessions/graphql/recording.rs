//! K1 `RecordingState` and the loader behind `Session.recording` (RA-040, G-5, API-005).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::collections::HashMap;

use async_graphql::{Enum, Error as GraphQlError, dataloader::Loader};
use sqlx::PgPool;

use crate::{
    api_errors::internal,
    ids::SessionId,
    sessions::recording::{RecordingState, states},
};

/// Mirrors the recording's state as a session shows it; the domain enum stays free of GraphQL.
#[derive(Clone, Copy, Debug, Enum, PartialEq, Eq)]
#[graphql(remote = "RecordingState", name = "RecordingState")]
pub enum RecordingStateValue {
    Off,
    On,
    Stopped,
}

/// The recording state of each session, for `Session.recording`: one query per batch of
/// session ids (API-005). A session with no recording row is `OFF`.
pub struct RecordingStates(pub(super) PgPool);

impl Loader<SessionId> for RecordingStates {
    type Value = RecordingStateValue;
    type Error = GraphQlError;

    async fn load(
        &self,
        keys: &[SessionId],
    ) -> Result<HashMap<SessionId, RecordingStateValue>, Self::Error> {
        let states = states(&self.0, keys).await.map_err(internal)?;
        Ok(states
            .into_iter()
            .map(|(session, state)| (session, state.into()))
            .collect())
    }
}
