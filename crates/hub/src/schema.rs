//! GraphQL schema root. Later packages merge domain parts into `Query`.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

pub struct Query;

#[Object]
impl Query {
    /// Liveness probe.
    async fn health(&self) -> &'static str {
        "ok"
    }
}

pub type HubSchema = Schema<Query, EmptyMutation, EmptySubscription>;

#[must_use]
pub fn build_schema() -> HubSchema {
    Schema::build(Query, EmptyMutation, EmptySubscription).finish()
}
