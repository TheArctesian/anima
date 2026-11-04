use std::{collections::BTreeMap, sync::Arc};

use async_graphql::value;

use super::{HubSchema, build_schema};
use crate::{clock::SystemClock, config};

fn real_schema() -> HubSchema {
    let config = config::from_env(&BTreeMap::new()).unwrap();
    build_schema(Arc::new(config), Arc::new(SystemClock))
}

const INTROSPECTION: &str = "{ __schema { queryType { name } } }";

/// Twelve levels deep through introspection, the only deep path the real schema has.
const TOO_DEEP: &str = "{ __schema { types { fields { type { ofType { ofType { ofType { ofType \
                        { ofType { ofType { name } } } } } } } } } } }";

#[cfg(debug_assertions)]
#[tokio::test]
async fn api_007_debug_build_answers_introspection_and_api_004_refuses_depth_11() {
    let ok = real_schema().execute(INTROSPECTION).await;
    assert_eq!(
        ok.data,
        value!({ "__schema": { "queryType": { "name": "Query" } } })
    );
    let deep = real_schema().execute(TOO_DEEP).await;
    assert!(deep.errors[0].message.contains("nested too deep"));
}

// The release side runs only under `cargo test --release`; `bun run gates` builds debug.
#[cfg(not(debug_assertions))]
#[tokio::test]
async fn api_007_release_build_answers_introspection_with_null() {
    let response = real_schema().execute(INTROSPECTION).await;
    assert_eq!(response.data, value!({ "__schema": null }));
}
