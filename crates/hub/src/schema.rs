//! GraphQL schema root: merges each domain's `Query` and `Mutation` (CQ-033, K3).

use std::sync::Arc;

use async_graphql::{EmptySubscription, MergedObject, Object, Schema};
use sqlx::PgPool;

use crate::{access, clock::Clock, config::Config, events::Bus};

#[derive(Default)]
pub struct Health;

#[Object(name = "HealthQuery")]
impl Health {
    /// Liveness probe. Needs a token like every operation except `login` (API-003).
    async fn health(&self) -> &'static str {
        "ok"
    }
}

#[derive(MergedObject, Default)]
pub struct Query(access::graphql::Query, Health);

#[derive(MergedObject, Default)]
pub struct Mutation(access::graphql::Mutation);

pub type HubSchema = Schema<Query, Mutation, EmptySubscription>;

/// Put each dependency into the schema data; resolvers read them with `ctx.data::<T>()`.
/// The access extension refuses every operation without a token except `login` (API-003).
#[must_use]
pub fn build_schema(
    config: Arc<Config>,
    clock: Arc<dyn Clock>,
    pool: PgPool,
    bus: Bus,
) -> HubSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(config)
        .data(clock)
        .data(pool)
        .data(bus)
        .extension(access::SignInRequired)
        .finish()
}
