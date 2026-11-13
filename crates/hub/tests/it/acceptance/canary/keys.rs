//! An API key made, shown once, used as a bearer, refused where D-23 refuses it, and used for a
//! `/metrics` scrape; and a key the hub never made (INT-003, D-23, D-24, LOG-007).

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;

use super::{
    Secrets,
    edge::{WholeHub, code},
};
use crate::hub::USERNAME;

const CREATE: &str =
    "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret key { id } } }";

/// Makes a READ key with the operator's `token` and uses it on each path.
pub async fn api_key(hub: &WholeHub, token: &str, now: DateTime<Utc>, secrets: &mut Secrets) {
    let input = json!({ "name": "scraper", "scope": "READ", "madeFor": "USER",
                        "expiresAt": now + TimeDelta::days(30) });
    let made = hub.data(token, CREATE, json!({ "input": input })).await;
    let secret = made["createApiKey"]["secret"].as_str().unwrap().to_owned();
    secrets.keep("api-key", &secret);

    let me = hub.data(&secret, "{ me { username } }", json!({})).await;
    assert_eq!(me["me"]["username"], USERNAME);
    let refused = hub
        .as_bearer(&secret, CREATE, json!({ "input": input }))
        .await;
    assert_eq!(code(&refused), "FORBIDDEN", "D-23: a key makes no key");
    let scraped = hub.get_as("/metrics", &secret).await;
    assert_eq!(scraped.status, StatusCode::OK);

    let unknown = format!("anima_{}", secrets.plant("unknown-api-key"));
    let refused = hub
        .as_bearer(&unknown, "{ me { username } }", json!({}))
        .await;
    assert_eq!(code(&refused), "UNAUTHENTICATED");
    let refused = hub.get_as("/metrics", &unknown).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
}
