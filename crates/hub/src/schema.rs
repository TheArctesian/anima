//! GraphQL schema root: merges each domain's `Query`, `Mutation` and `Subscription` (CQ-033, K3).
//!
//! A domain joins with one line in each root it has and one block of its data in
//! `build_schema`; a domain with state `main.rs` builds once (such as `sessions::Live`) also
//! adds a `Dependencies` field.

use std::{sync::Arc, time::Duration};

use async_graphql::{MergedObject, MergedSubscription, Schema, SchemaBuilder};
use sqlx::PgPool;

use crate::{
    access, builds, checks,
    clock::Clock,
    config::Config,
    credentials::Vault,
    devices,
    events::{
        self, Bus,
        process_log::{InternalErrors, RequestSpan},
    },
    maintenance, metrics, notifications, retention,
    sessions::{self, Live},
    targets::AddressPolicy,
    trust, vpn, web_interfaces,
};

// One line per domain in each root (K3).
#[derive(MergedObject, Default)]
pub struct Query(
    access::graphql::Query,
    builds::graphql::Query,
    checks::graphql::Query,
    devices::graphql::Query,
    events::graphql::Query,
    maintenance::graphql::Query,
    notifications::graphql::Query,
    retention::graphql::Query,
    sessions::graphql::Query,
    sessions::RecordingsQuery,
    vpn::graphql::Query,
);

#[derive(MergedObject, Default)]
pub struct Mutation(
    access::graphql::Mutation,
    builds::graphql::Mutation,
    builds::graphql::BuilderMutation,
    checks::graphql::Mutation,
    devices::graphql::Mutation,
    maintenance::graphql::Mutation,
    metrics::graphql::Mutation,
    notifications::graphql::Mutation,
    retention::graphql::Mutation,
    sessions::graphql::Mutation,
    sessions::FilesMutation,
    sessions::RecordingsMutation,
    sessions::WakeMutation,
    trust::graphql::Mutation,
    vpn::graphql::Mutation,
    vpn::graphql::RouteMutation,
    web_interfaces::graphql::Mutation,
);

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    builds::graphql::Subscription,
    checks::graphql::Subscription,
    events::graphql::Subscription,
    metrics::graphql::Subscription,
    sessions::graphql::Subscription,
    vpn::graphql::Subscription,
);

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
    /// The hub's one `Feed`: the collector publishes on it and `metricsAdded` reads it, so
    /// `main.rs` gives the collector a clone of this same value.
    pub metrics_feed: metrics::Feed,
    /// The hub's one build `Updates`: the lease upkeep publishes on it and `buildJobUpdated`
    /// reads it, so `main.rs` gives the upkeep this same value.
    pub build_updates: builds::Updates,
    /// The release bucket `buildUploadUrls` signs for; `None` when publishing is off (BLD-060,
    /// R-108). `main.rs` gives the publish upkeep the same value.
    pub bucket: Option<Arc<builds::Bucket>>,
    /// The hub's one VPN `Feed`: the reader publishes on it and `vpn` and `vpnChanged` read
    /// it, so the upkeep gives the reader this same value.
    pub vpn_feed: vpn::Feed,
    /// The hub's one VPN broker; the reader and the VPN mutations share its turn (BD-15).
    /// `None` when `ANIMA_VPN_BROKER` is unset.
    pub vpn_broker: Option<vpn::Broker>,
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
        metrics_feed,
        build_updates,
        bucket,
        vpn_feed,
        vpn_broker,
    } = dependencies;
    let builder = harden(
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
    .data(config.clone())
    .data(clock.clone())
    .data(pool.clone())
    .data(bus)
    .data(vault);
    // One block per domain: its loaders (R-014) and its state.
    with_devices_loaders(builder, &pool, &clock)
        // checks
        .data(checks::graphql::check_status_loader(pool.clone(), clock.clone()).delay(BATCH_WINDOW))
        .data(checks::graphql::last_heartbeat_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(checks::graphql::heartbeats_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(checks::graphql::check_uptime_loader(pool.clone(), clock).delay(BATCH_WINDOW))
        .data(checks::graphql::check_notifications_loader(pool.clone()).delay(BATCH_WINDOW))
        // notifications
        .data(notifications::graphql::last_delivery_loader(pool.clone()).delay(BATCH_WINDOW))
        // trust: the accepted keys and certificates, and what the connection tests of `live` saw
        .data(trust::graphql::host_key_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(trust::graphql::server_certificate_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(live.sightings().clone())
        // sessions (R-032): `Session.recording`, and the recording storage `startSession` checks
        // and `deleteRecording` removes files from
        .data(sessions::graphql::recording_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(recording(&config))
        .data(live)
        .data(policy)
        // metrics
        .data(metrics_feed)
        // builds
        .data(build_updates)
        .data(bucket)
        // vpn
        .data(vpn_feed)
        .data(vpn_broker)
        .finish()
}

type Builder = SchemaBuilder<Query, Mutation, Subscription>;

/// The devices loaders (R-014), with the devices fields that join checks and metrics
/// (API-005).
fn with_devices_loaders(builder: Builder, pool: &PgPool, clock: &Arc<dyn Clock>) -> Builder {
    builder
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
        // devices fields that join metrics (API-005)
        .data(devices::graphql::readings_loader(pool.clone(), clock.clone()).delay(BATCH_WINDOW))
        .data(devices::graphql::gpus_loader(pool.clone()).delay(BATCH_WINDOW))
        .data(devices::graphql::stale_loader(pool.clone()).delay(BATCH_WINDOW))
}

/// The recording storage from the hub's configuration (K7 `ANIMA_RECORDING_ROOT`,
/// `ANIMA_RECORDING_MAX_BYTES`): what `startSession` checks, the tunnels record to and
/// `deleteRecording` removes files from.
#[must_use]
pub(crate) fn recording(config: &Config) -> sessions::Recording {
    sessions::Recording {
        root: config.recording_root.clone(),
        max_bytes: config.recording_max_bytes,
    }
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
