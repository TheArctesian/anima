//! The files GraphQL part (K1 `openFiles`, WP-27 D-8): merged into the root `Mutation` behind
//! the same guard as the sessions part.

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use sqlx::PgPool;

use super::opening::open_files;
use crate::{
    api_errors::{invalid, schema_dependency},
    clock::Clock,
    fleet::{
        access::Authenticated,
        events::{ClientAddr, Operator},
        sessions::{Actor, FieldError, Live, Ticket, graphql::to_graphql},
    },
    ids::SessionId,
};

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`; the name only keeps it apart from the other parts.
#[Object(name = "FilesMutation")]
impl Mutation {
    /// A single-use ticket for the files tunnel of a live SSH session (K4 §9, API-009).
    #[graphql(guard = "Authenticated")]
    async fn open_files(&self, ctx: &Context<'_>, session_id: ID) -> Result<Ticket> {
        let session = session_id.parse::<SessionId>().map_err(|_| {
            invalid(&[FieldError::new(
                "sessionId",
                "That session id is not valid.",
            )])
        })?;
        let by = Actor {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        };
        open_files(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            &by,
            session,
        )
        .await
        .map_err(to_graphql)
    }
}
