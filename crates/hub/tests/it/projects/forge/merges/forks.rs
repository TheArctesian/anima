//! PRO-013, K-E1 Section 13.3: a pull request from a fork merges alone, only when the operator
//! typed its head repository exactly, and never for a request that declares an AI agent. A
//! request that declares an agent may merge one from the repository itself (K1).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    FORK, MERGE, as_agent, changes_sent, forked, input, merge, merged, methods, reads, script, sent,
};
use crate::{
    hub::error,
    projects::forge::{changes::audits, forge},
};

const UNCONFIRMED: &str =
    "It is from a fork: set forkConfirmed to its head repository to merge it (PRO-013).";

fn forbidden(response: &Value, words: &str) {
    assert_eq!(
        error(response),
        (words.to_owned(), "FORBIDDEN".to_owned()),
        "{response}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_013_a_fork_merges_only_with_its_head_repository_typed(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/12", &forked(12));

    for typed in [
        Value::Null,
        json!("octo/widgets"),
        json!("Stranger/Widgets"),
        json!(""),
    ] {
        let response = merge(&forge, 12, &json!({ "forkConfirmed": typed })).await;
        forbidden(&response, UNCONFIRMED);
    }
    assert_eq!(changes_sent(&forge), 0, "nothing");
    assert!(audits(&pool).await.is_empty(), "nothing");

    reads(&forge, 12, &forked(12), &merged(12));
    super::merges(&forge, 12);
    let response = merge(&forge, 12, &json!({ "forkConfirmed": FORK })).await;
    assert_eq!(
        response["data"]["mergePullRequest"]["state"], "MERGED",
        "{response}"
    );
    assert_eq!(sent(&forge, 12).len(), 1);
    assert_eq!(audits(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_013_a_fork_whose_head_repository_is_gone_never_merges(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    let mut gone = forked(12);
    gone["head"]["repo"] = Value::Null;
    forge.ok("/pulls/12", &gone);

    let response = merge(&forge, 12, &json!({ "forkConfirmed": "" })).await;

    forbidden(&response, UNCONFIRMED);
    assert_eq!(changes_sent(&forge), 0, "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_013_a_request_that_declares_an_agent_never_merges_a_fork(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/12", &forked(12));
    let variables = input(&forge, 12, &json!({ "forkConfirmed": FORK }));

    let response = as_agent(&forge, &forge.kit.token, MERGE, variables).await;

    forbidden(
        &response,
        "An AI agent cannot merge a pull request from a fork. The operator merges it (PRO-013).",
    );
    assert_eq!(changes_sent(&forge), 0, "nothing");
    assert!(audits(&pool).await.is_empty(), "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_request_that_declares_an_agent_merges_one_of_the_repository_and_says_so(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    script(&forge, 3);
    let variables = input(&forge, 3, &json!({}));

    let response = as_agent(&forge, &forge.kit.token, MERGE, variables).await;

    assert_eq!(
        response["data"]["mergePullRequest"]["state"], "MERGED",
        "{response}"
    );
    let entries = audits(&pool).await;
    assert_eq!(
        (&entries[0]["actor_kind"], &entries[0]["token_kind"]),
        (&json!("agent"), &json!("login"))
    );
}
