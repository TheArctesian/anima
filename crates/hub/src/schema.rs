//! GraphQL schema root. Later packages merge domain parts into `Query`.

use std::sync::Arc;

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

use crate::{clock::Clock, config::Config};

pub struct Query;

#[Object]
impl Query {
    /// Liveness probe.
    async fn health(&self) -> &'static str {
        "ok"
    }
}

pub type HubSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Put each dependency into the schema data; resolvers read them with `ctx.data::<T>()`.
#[must_use]
pub fn build_schema(config: Arc<Config>, clock: Arc<dyn Clock>) -> HubSchema {
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(config)
        .data(clock)
        .finish()
}
