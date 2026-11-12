//! The sessions GraphQL part: root `Query`, `Mutation` and `Subscription` (K1; RA-008,
//! RA-034, API-003, API-006, API-009). `acceptHostKey` and `acceptServerCertificate` are in
//! `trust::graphql`.
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type.

mod change;
mod connection;
pub(super) mod connection_test;
pub(super) mod error;
mod session;
mod ticket;

use std::sync::Arc;

use async_graphql::{
    Context, ID, Object, Result, Subscription as GraphQLSubscription, futures_util::stream::Stream,
};
use sqlx::PgPool;

/// For the files part's resolver (WP-27 D-8): the same errors and schema data.
pub(super) use self::error::to_graphql;

use self::{
    change::session_changes,
    connection::{DEFAULT_FIRST, SessionConnection, sessions_args},
    connection_test::ConnectionTestInput,
    ticket::StartSessionInput,
};
use super::{
    Actor, FieldError, Live, Session, SessionChange, Ticket,
    connection_test::{ConnectionTest, TestRequest, test_connection},
    end::end_session,
    start, store,
};
use crate::{
    access::Authenticated,
    api_errors::{invalid, schema_dependency},
    clock::Clock,
    credentials::Vault,
    events::{ClientAddr, Operator},
    ids::SessionId,
};

/// Who acts, from where and when, as the schema data of a signed-in request gives it.
fn actor(ctx: &Context<'_>) -> Result<Actor> {
    Ok(Actor {
        operator: schema_dependency::<Operator>(ctx)?.clone(),
        client: *schema_dependency::<ClientAddr>(ctx)?,
        at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
    })
}

#[derive(Default)]
pub struct Query;

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "SessionsQuery")]
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

/// Merged into the root `Mutation`.
#[Object(name = "SessionsMutation")]
impl Mutation {
    /// Starts a session and returns its single-use tunnel ticket (K4 §1, API-009).
    #[graphql(guard = "Authenticated")]
    async fn start_session(&self, ctx: &Context<'_>, input: StartSessionInput) -> Result<Ticket> {
        let mut request = input.into_request().map_err(|fields| invalid(&fields))?;
        request.wake = Some(ticket::waking(ctx, request.device).await?);
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
        let by = actor(ctx)?;
        end_session(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            session,
            &by,
        )
        .await
        .map_err(to_graphql)
    }

    /// Tests SSH, RDP or VNC to a saved device or to the form's values (DEV-013). For SSH a
    /// credential is sent only to the device's accepted host key (SEC-006); an RDP or VNC test
    /// sends none, and RDP reads the server certificate (SEC-007).
    #[graphql(guard = "Authenticated")]
    async fn test_connection(
        &self,
        ctx: &Context<'_>,
        input: ConnectionTestInput,
    ) -> Result<ConnectionTest> {
        let subject = input.into_subject().map_err(|fields| invalid(&fields))?;
        let request = TestRequest {
            subject,
            at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        };
        test_connection(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Vault>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            request,
        )
        .await
        .map_err(to_graphql)
    }
}

#[derive(Default)]
pub struct Subscription;

/// Merged into the root `Subscription`.
#[GraphQLSubscription(name = "SessionsSubscription")]
impl Subscription {
    /// Every start and end of a session, from every client, once it is committed (RA-034).
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn session_changed(
        &self,
        ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = SessionChange>> {
        Ok(session_changes(schema_dependency::<Live>(ctx)?.changes()))
    }
}
