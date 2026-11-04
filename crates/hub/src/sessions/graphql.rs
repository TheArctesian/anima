//! The sessions GraphQL part: root `Query` and `Mutation` (K1; RA-008, API-003, API-006,
//! API-009).
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type. A later task adds `testConnection` and `acceptHostKey`.

mod connection;
mod error;
mod session;
mod ticket;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use sqlx::PgPool;

use self::{
    connection::{DEFAULT_FIRST, SessionConnection, sessions_args},
    error::{invalid, schema_dependency, to_graphql},
    ticket::StartSessionInput,
};
use super::{Actor, FieldError, Live, Session, Ticket, end::end_session, start, store};
use crate::{
    access::Authenticated,
    clock::Clock,
    events::{ClientAddr, Operator},
    ids::SessionId,
};

#[derive(Default)]
pub struct Query;

#[Object]
impl Query {
    /// Sessions newest first, live and ended (RA-008).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.sessions"
    )]
    async fn sessions(
        &self,
        ctx: &Context<'_>,
        device_id: Option<ID>,
        live: Option<bool>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<SessionConnection> {
        let args = sessions_args(
            device_id.as_ref(),
            live,
            first.unwrap_or(DEFAULT_FIRST),
            after.as_deref(),
        )
        .map_err(|fields| invalid(&fields))?;
        let page = store::page(
            schema_dependency::<PgPool>(ctx)?,
            args.filter,
            args.size,
            args.after,
        )
        .await
        .map_err(to_graphql)?;
        Ok(SessionConnection::from(page))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object]
impl Mutation {
    /// Starts a session and returns its single-use tunnel ticket (K4 §1, API-009).
    #[graphql(guard = "Authenticated")]
    async fn start_session(&self, ctx: &Context<'_>, input: StartSessionInput) -> Result<Ticket> {
        let request = input.into_request().map_err(|fields| invalid(&fields))?;
        start(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Arc<dyn Clock>>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            schema_dependency::<Operator>(ctx)?,
            *schema_dependency::<ClientAddr>(ctx)?,
            request,
        )
        .await
        .map_err(to_graphql)
    }

    /// Ends a session with `OPERATOR`, live or pending; an ended one is returned unchanged
    /// (D-10).
    #[graphql(guard = "Authenticated")]
    async fn end_session(&self, ctx: &Context<'_>, id: ID) -> Result<Session> {
        let session = id
            .parse::<SessionId>()
            .map_err(|_| invalid(&[FieldError::new("id", "That session id is not valid.")]))?;
        let by = Actor {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        };
        end_session(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            session,
            &by,
        )
        .await
        .map_err(to_graphql)
    }
}
