use async_graphql::{EmptySubscription, Schema, value};

use std::{collections::BTreeMap, path::PathBuf};

use super::{Mutation, Query, harden, production_build, recording};
use crate::config::from_env;

/// The real root types under `harden`, as `build_schema` applies it, without the access
/// backstop: these tests are about the limits, not about sign-in (that is `access`'s and the
/// `it` crate's).
fn hardened() -> Schema<Query, Mutation, EmptySubscription> {
    harden(
        Schema::build(Query::default(), Mutation::default(), EmptySubscription),
        production_build(),
    )
    .finish()
}

const INTROSPECTION: &str = "{ __schema { queryType { name } } }";

#[cfg(debug_assertions)]
/// Twelve levels deep through introspection, the only deep path the real schema has.
const TOO_DEEP: &str = "{ __schema { types { fields { type { ofType { ofType { ofType { ofType \
                        { ofType { ofType { name } } } } } } } } } } }";

#[cfg(debug_assertions)]
#[tokio::test]
async fn api_007_debug_build_answers_introspection_and_api_004_refuses_depth_11() {
    let ok = hardened().execute(INTROSPECTION).await;
    assert_eq!(
        ok.data,
        value!({ "__schema": { "queryType": { "name": "Query" } } })
    );
    let deep = hardened().execute(TOO_DEEP).await;
    assert!(deep.errors[0].message.contains("nested too deep"));
}

// The release side runs only under `cargo test --release -p anima-hub schema`, which the gates
// do not run. Only that command catches a mutation of `production_build()`, so it is a manual
// check until it is gated.
#[cfg(not(debug_assertions))]
#[tokio::test]
async fn api_007_release_build_answers_introspection_with_null() {
    let response = hardened().execute(INTROSPECTION).await;
    assert_eq!(response.data, value!({ "__schema": null }));
}

#[test]
fn d_14_the_recording_storage_is_the_configured_root_and_cap() {
    let mut config = from_env(&BTreeMap::new()).unwrap();
    config.recording_root = Some(PathBuf::from("/var/lib/anima/recordings"));
    config.recording_max_bytes = 1_048_576;
    let storage = recording(&config);
    assert_eq!(
        storage.root,
        Some(PathBuf::from("/var/lib/anima/recordings"))
    );
    assert_eq!(storage.max_bytes, 1_048_576);
}
