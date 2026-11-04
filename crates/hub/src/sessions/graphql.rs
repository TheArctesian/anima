//! The sessions GraphQL part: root `Query` (K1; RA-008, API-003, API-006).
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type. Later tasks add `Mutation` (`startSession`, `endSession`, `testConnection`,
//! `acceptHostKey`).

mod connection;
mod error;
mod session;

use async_graphql::{Context, ID, Object, Result};
use sqlx::PgPool;

use self::{
    connection::{DEFAULT_FIRST, SessionConnection, sessions_args},
    error::{invalid, schema_dependency, to_graphql},
};
use super::store;
use crate::access::Authenticated;

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
