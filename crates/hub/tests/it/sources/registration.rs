//! Connect GitHub through the hub (PRJ-039 to PRJ-042, K-E1 Section 7a): the attempt's API,
//! and the browser's way through the hub's pages and GitHub's redirects, played against the
//! loopback stub. Never the real GitHub.

mod api;
mod browser;
mod flow;
mod leaks;
mod pages;
mod refusals;

use serde_json::{Value, json};

use super::Kit;

pub const START: &str = "mutation($input: StartGithubConnectInput) { startGithubConnect(input: $input) { \
    id phase startedBy startedAt expiresAt pageUrl appId appSlug ownerLogin installUrl \
    appSettingsUrl failure message account { id label } } }";
pub const OPEN: &str = "mutation($id: ID!) { openGithubConnect(id: $id) { url expiresAt } }";
pub const CANCEL: &str = "mutation($id: ID!) { cancelGithubConnect(id: $id) { id phase message } }";
pub const STATE: &str = "query { githubConnect { id phase failure message appSlug ownerLogin \
    installUrl appSettingsUrl account { id label credentialStored } } }";

/// Starts an attempt with `input`; returns the whole response.
pub async fn start_with(kit: &Kit, input: Value) -> Value {
    kit.hub
        .post(Some(&kit.token), START, json!({ "input": input }))
        .await
}

/// Starts an attempt for the operator's own account; returns its id.
pub async fn started(kit: &Kit) -> String {
    let response = start_with(kit, Value::Null).await;
    response["data"]["startGithubConnect"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("start failed: {response}"))
        .to_owned()
}

/// The one-time link of attempt `id`.
pub async fn link(kit: &Kit, id: &str) -> String {
    let response = kit
        .hub
        .post(Some(&kit.token), OPEN, json!({ "id": id }))
        .await;
    response["data"]["openGithubConnect"]["url"]
        .as_str()
        .unwrap_or_else(|| panic!("open failed: {response}"))
        .to_owned()
}
