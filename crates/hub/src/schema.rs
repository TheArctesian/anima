//! GraphQL schema root: merges each domain's `Query`, `Mutation` and `Subscription` (CQ-033, K3).
//!
//! A domain joins in two places: one line in each root it has, and one block of its data in
//! `build_schema`.

use std::sync::Arc;

use async_graphql::{MergedObject, MergedSubscription, Object, Schema, SchemaBuilder};
use sqlx::PgPool;

use crate::{
    access,
    clock::Clock,
    config::Config,
    credentials::Vault,
    devices,
    events::{
        self, Bus,
        process_log::{InternalErrors, RequestSpan},
    },
    sessions::{self, Live},
};

#[derive(Default)]
pub struct Health;

#[Object(name = "HealthQuery")]
impl Health {
    /// Liveness probe. Needs a token like every operation except `login` (API-003).
    async fn health(&self) -> &'static str {
        "ok"
    }
}

// One line per domain in each root (K3).
#[derive(MergedObject, Default)]
pub struct Query(
    access::graphql::Query,
    devices::graphql::Query,
    events::graphql::Query,
    sessions::graphql::Query,
    Health,
);

#[derive(MergedObject, Default)]
pub struct Mutation(
    access::graphql::Mutation,
    devices::graphql::Mutation,
    sessions::graphql::Mutation,
);

#[derive(MergedSubscription, Default)]
pub struct Subscription(events::graphql::Subscription);

pub type HubSchema = Schema<Query, Mutation, Subscription>;

/// What `main.rs` builds once and the schema hands to resolvers as context data (R-002).
pub struct Dependencies {
    pub config: Arc<Config>,
    pub clock: Arc<dyn Clock>,
    pub pool: PgPool,
    pub bus: Bus,
    /// Registered as `Vault` itself, as K3 writes `.data(vault)`; a router that needs it too
    /// gets its own clone.
    pub vault: Vault,
    /// The hub's one `Live`: the tunnel router gets a clone of this same value, or no ticket
    /// the schema issues can be redeemed (R-032).
    pub live: Live,
}

/// Put each dependency into the schema data; resolvers read them with `ctx.data::<T>()`.
/// The access extension refuses every operation without a token except `login` (API-003).
/// Validation errors never suggest field names ("Did you mean"), so a signed-out client
/// learns nothing about the schema from a typo (API-003, API-007).
#[must_use]
pub fn build_schema(dependencies: Dependencies) -> HubSchema {
    let Dependencies {
        config,
        clock,
        pool,
        bus,
        vault,
        live,
    } = dependencies;
    harden(
        Schema::build(
            Query::default(),
            Mutation::default(),
            Subscription::default(),
        ),
        production_build(),
    )
    .disable_suggestions()
    .extension(access::SignInRequired)
    // Shared dependencies (K3 wiring).
    .data(config)
    .data(clock)
    .data(pool.clone())
    .data(bus)
    .data(vault)
    // One block per domain: its loaders (R-014) and its state.
    // devices
    .data(devices::graphql::site_loader(pool.clone()))
    .data(devices::graphql::devices_by_site_loader(pool.clone()))
    .data(devices::graphql::credential_loader(pool.clone()))
    // sessions (R-032)
    .data(sessions::graphql::host_key_loader(pool.clone()))
    .data(live)
    .finish()
}

/// API-007: release builds are production builds; debug builds keep introspection for tooling.
const fn production_build() -> bool {
    !cfg!(debug_assertions)
}

#[cfg(test)]
mod tests;

/// API-004: deepest query accepted.
const MAX_DEPTH: usize = 10;
/// API-004: most fields one query may select (each field costs 1). The operator's own client is
/// the only caller. Assuming its largest screen selects under 100 fields, 500 leaves room for
/// growth and still stops a query that fans out to thousands of fields. Revisit if a real
/// query comes near it.
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
