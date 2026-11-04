//! The GraphQL field guard that refuses requests without a signed-in operator (API-003).

use std::future::{Future, ready};

use async_graphql::{Context, Guard};

use super::{Error, Operator};

/// GraphQL guard: requires a valid token; puts `Operator` in the context (API-003).
///
/// A guard cannot add context data (D-6): the HTTP and WebSocket edges resolve the bearer
/// token and put the `Operator` in the request data. This guard passes only when it is there,
/// and refuses with `UNAUTHENTICATED` otherwise.
pub struct Authenticated;

impl Guard for Authenticated {
    fn check(&self, ctx: &Context<'_>) -> impl Future<Output = async_graphql::Result<()>> + Send {
        let verdict = match ctx.data_opt::<Operator>() {
            Some(_) => Ok(()),
            None => Err(Error::Unauthenticated.to_graphql()),
        };
        ready(verdict)
    }
}
