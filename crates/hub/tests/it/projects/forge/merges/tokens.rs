//! D-23, R-100, K-E1 Section 13.5: both merge roots are login or browser token only. A read key,
//! a write key made for a user or an agent, and a signed-out request are refused before
//! anything is read or sent; a login token merges.

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{MERGE, MERGE_MANY, changes_sent, input, script};
use crate::{
    hub::{error, start, unauthenticated},
    projects::forge::{Forge, changes::audits, forge, sha},
};

const KIND_REFUSED: &str = "This token cannot do that. Sign in to the hub to do it.";

async fn key(forge: &Forge, scope: &str, made_for: &str) -> String {
    let query = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "input": { "name": format!("{scope}-{made_for}"), "scope": scope,
        "madeFor": made_for, "expiresAt": start() + TimeDelta::days(30) } });
    let response = forge.kit.post(query, input).await;
    response["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("no key: {response}"))
        .to_owned()
}

fn calls(forge: &Forge) -> [(&'static str, Value); 2] {
    let many = json!({ "pullRequests": [{ "repository": forge.repository, "number": 3,
        "expectedHeadSha": sha(3) }], "method": "SQUASH" });
    [(MERGE, input(forge, 3, &json!({}))), (MERGE_MANY, many)]
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_an_api_key_of_any_scope_is_refused_both_merges_and_a_login_token_is_not(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    script(&forge, 3);
    let keys = [
        key(&forge, "READ", "USER").await,
        key(&forge, "WRITE", "USER").await,
        key(&forge, "WRITE", "AGENT").await,
    ];
    let reads_before = forge.live_reads();
    for token in &keys {
        for (query, variables) in calls(&forge) {
            let refused = forge.kit.hub.post(Some(token), query, variables).await;
            assert_eq!(
                error(&refused),
                (KIND_REFUSED.to_owned(), "FORBIDDEN".to_owned()),
                "{query}"
            );
        }
    }
    for (query, variables) in calls(&forge) {
        let signed_out = forge.kit.hub.post(None, query, variables).await;
        assert_eq!(error(&signed_out), unauthenticated(), "{query}");
    }
    assert_eq!(forge.live_reads(), reads_before, "nothing was read");
    assert_eq!(changes_sent(&forge), 0, "nothing was sent");
    assert!(audits(&pool).await.is_empty(), "nothing");

    let [(one, variables), _] = calls(&forge);
    let response = forge
        .kit
        .hub
        .post(Some(&forge.kit.token), one, variables)
        .await;
    assert_eq!(
        response["data"]["mergePullRequest"]["state"], "MERGED",
        "{response}"
    );
}
