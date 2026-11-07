//! GraphQL schema root: merges each domain's `Query`, `Mutation` and `Subscription` (CQ-033, K3).
//!
//! A domain joins with one line in each root it has and one block of its data in
//! `build_schema`; a domain with state `main.rs` builds once (such as `sessions::Live`) also
//! adds a `Dependencies` field.

use std::{sync::Arc, time::Duration};

use async_graphql::{MergedObject, MergedSubscription, Schema, SchemaBuilder};
use sqlx::PgPool;

use crate::{
    access, checks,
    clock::Clock,
    config::Config,
    credentials::Vault,
    devices,
    events::{
        self, Bus,
        process_log::{InternalErrors, RequestSpan},
    },
    maintenance,
    sessions::{self, Live},
    targets::AddressPolicy,
    web_interfaces,
};

// One line per domain in each root (K3).
#[derive(MergedObject, Default)]
pub struct Query(
    access::graphql::Query,
    checks::graphql::Query,
    devices::graphql::Query,
    events::graphql::Query,
    maintenance::graphql::Query,
    sessions::graphql::Query,
);

#[derive(MergedObject, Default)]
pub struct Mutation(
    access::graphql::Mutation,
    checks::graphql::Mutation,
    devices::graphql::Mutation,
    maintenance::graphql::Mutation,
    sessions::graphql::Mutation,
    sessions::FilesMutation,
    sessions::WakeMutation,
    web_interfaces::graphql::Mutation,
);

#[derive(MergedSubscription, Default)]
pub struct Subscription(checks::graphql::Subscription, events::graphql::Subscription);

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
    /// The deny list every SSH link the schema opens checks (`wakeDevice`; P2-004).
    pub policy: AddressPolicy,
}

/// Put each dependency into the schema data, with the devices loaders (WP-06 H-2);
/// resolvers read them with `ctx.data::<T>()`. The HTTP edge adds the `Operator` and the
/// `ClientAddr` to each request (D-6).
/// The access extension refuses every operation without a token except `login` (API-003).
/// Validation errors never suggest field names, and signed out they become one
/// `UNAUTHENTICATED`, so a signed-out client learns nothing about the schema (API-003,
/// API-007).
#[must_use]
pub fn build_schema(dependencies: Dependencies) -> HubSchema {
    let Dependencies {
        config,
        clock,
        pool,
        bus,
        vault,
        live,
        policy,
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
    .data(clock.clone())
    .data(pool.clone())
    .data(bus)
    .data(vault)
    // One block per domain: its loaders (R-014) and its state.
    // devices
    .data(devices::graphql::site_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::devices_by_site_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::credential_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::rdp_credential_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::vnc_credential_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::device_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::web_interfaces_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::web_interface_suggestions_loader(pool.clone()).delay(BATCH_WINDOW))
    // devices fields that join checks (API-005)
    .data(devices::graphql::status_loader(pool.clone(), clock.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::uptime_loader(pool.clone(), clock.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::checks_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(devices::graphql::resolved_ip_loader(pool.clone()).delay(BATCH_WINDOW))
    // checks
    .data(checks::graphql::check_status_loader(pool.clone(), clock.clone()).delay(BATCH_WINDOW))
    .data(checks::graphql::last_heartbeat_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(checks::graphql::heartbeats_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(checks::graphql::check_uptime_loader(pool.clone(), clock).delay(BATCH_WINDOW))
    // sessions (R-032)
    .data(sessions::graphql::host_key_loader(pool.clone()).delay(BATCH_WINDOW))
    .data(live)
    .data(policy)
    .finish()
}

/// API-007: release builds are production builds; debug builds keep introspection for tooling.
const fn production_build() -> bool {
    !cfg!(debug_assertions)
}

#[cfg(test)]
mod tests;

/// API-005: how long every loader waits to gather keys. A device page nests several loaders,
/// and the 1 ms default splits their batches once a page holds a few hundred rows (342
/// statements for 163 devices in a debug build, against 15 for 13); each split repeats the
/// reads of a batch. 10 ms costs that much latency per nesting level.
const BATCH_WINDOW: Duration = Duration::from_millis(10);

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
