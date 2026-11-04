//! GraphQL schema root. Later packages merge domain parts into `Query`.

use std::sync::Arc;

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, SchemaBuilder};

use crate::{
    clock::Clock,
    config::Config,
    events::process_log::{InternalErrors, RequestSpan},
};

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
    harden(
        Schema::build(Query, EmptyMutation, EmptySubscription),
        !cfg!(debug_assertions),
    )
    .data(config)
    .data(clock)
    .finish()
}

/// API-004: deepest query accepted.
const MAX_DEPTH: usize = 10;
/// API-004: most fields one query may select (each field costs 1). The operator's own client is
/// the only caller; its largest screen selects well under 100 fields, so 500 leaves room for
/// growth and still stops a query that fans out to thousands of fields.
const MAX_COMPLEXITY: usize = 500;

/// Applies the query limits (API-004), the process-log extensions (LOG-002, API-010) and, when
/// `production` is set, turns introspection off (API-007; there is no playground route).
#[must_use]
pub fn harden<Q, M, S>(
    builder: SchemaBuilder<Q, M, S>,
    production: bool,
) -> SchemaBuilder<Q, M, S> {
    let builder = builder
        .limit_depth(MAX_DEPTH)
        .limit_complexity(MAX_COMPLEXITY)
        .extension(RequestSpan)
        .extension(InternalErrors);
    if production {
        builder.disable_introspection()
    } else {
        builder
    }
}
