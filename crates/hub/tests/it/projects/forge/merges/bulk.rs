//! PRJ-033, K-E1 Section 13.3: a bulk merge takes each pull request in the order given, one at a
//! time; a failure does not stop the rest; only CLEAN ones merge and never one from a fork; the
//! hub stops starting merges 90 seconds after the request began. One audit entry for each merge
//! that was sent.

use std::{sync::Arc, time::Duration};

use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::Semaphore;

use super::{
    MERGE_MANY, changes_sent, clean, forked, merge_sha, merged, merges, methods, open, sent,
};
use crate::{
    hub::error,
    projects::forge::{BASE, Forge, changes::audits, forge, reply, sha},
};

fn target(forge: &Forge, number: u64) -> Value {
    json!({ "repository": forge.repository, "number": number, "expectedHeadSha": sha(number) })
}

async fn merge_many(forge: &Forge, targets: Vec<Value>) -> Value {
    let variables = json!({ "pullRequests": targets, "method": "MERGE" });
    forge.kit.post(MERGE_MANY, variables).await
}

/// `(number, outcome, mergeSha)` of each result, in order.
fn outcomes(response: &Value) -> Vec<(u64, String, Value)> {
    response["data"]["mergePullRequests"]
        .as_array()
        .unwrap_or_else(|| panic!("no results: {response}"))
        .iter()
        .map(|r| {
            let outcome = r["outcome"].as_str().unwrap().to_owned();
            (
                r["number"].as_u64().unwrap(),
                outcome,
                r["mergeSha"].clone(),
            )
        })
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_each_pull_request_has_its_outcome_in_order_and_a_failure_does_not_stop_the_rest(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    methods(&forge);
    for number in [1, 5, 7] {
        forge.ok(&format!("/pulls/{number}"), &clean(number));
        merges(&forge, number);
    }
    forge.ok("/pulls/2", &merged(2));
    forge.ok("/pulls/3", &forked(3));
    forge.ok("/pulls/4", &clean(4));
    forge.ok("/pulls/6", &open(6, "unstable", json!(true)));
    let path = format!("{BASE}/pulls/5/merge");
    (forge.kit.stub).reply_to("PUT", &path, reply(500, &[], "{}"));
    let mut moved = target(&forge, 4);
    moved["expectedHeadSha"] = json!(sha(44));
    let elsewhere = json!({ "repository": "0190a000-0000-7000-8000-0000000000aa", "number": 8,
        "expectedHeadSha": sha(8) });
    let targets = vec![
        target(&forge, 1),
        target(&forge, 2),
        target(&forge, 3),
        moved,
        target(&forge, 5),
        target(&forge, 6),
        target(&forge, 7),
        elsewhere,
        target(&forge, 9),
    ];

    let response = merge_many(&forge, targets).await;

    let merged_as = json!(merge_sha());
    let expected = [
        (1, "MERGED", merged_as.clone()),
        (2, "ALREADY_MERGED", Value::Null),
        (3, "FORK_REFUSED", Value::Null),
        (4, "HEAD_MOVED", Value::Null),
        (5, "UNKNOWN", Value::Null),
        (6, "NOT_MERGEABLE", Value::Null),
        (7, "MERGED", merged_as),
        (8, "NOT_FOUND", Value::Null),
        (9, "NOT_FOUND", Value::Null),
    ]
    .map(|(n, outcome, sha)| (n, outcome.to_owned(), sha));
    assert_eq!(outcomes(&response), expected, "{response}");
    let results = response["data"]["mergePullRequests"].as_array().unwrap();
    assert_eq!(results[0]["repository"], forge.repository.as_str());
    assert_eq!(
        results[2]["message"],
        "It is from a fork: merge it alone, after you type its head repository (PRO-013)."
    );
    let puts: Vec<usize> = [1, 3, 4, 5, 6, 7].map(|n| sent(&forge, n).len()).to_vec();
    assert_eq!(
        puts,
        [1, 0, 0, 1, 0, 1],
        "sent only what the rule allowed, once"
    );
    let recorded: Vec<(Value, Value)> = audits(&pool)
        .await
        .iter()
        .map(|e| {
            (
                e["change"]["target"].clone(),
                e["change"]["outcome"].clone(),
            )
        })
        .collect();
    let expected = [(1, "done"), (5, "unknown"), (7, "done")]
        .map(|(n, outcome)| (json!(format!("pull_request#{n}")), json!(outcome)));
    assert_eq!(recorded, expected, "one entry for each merge sent");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_the_hub_starts_no_merge_90_seconds_after_the_request_began(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    for number in 1..=3 {
        forge.ok(&format!("/pulls/{number}"), &clean(number));
        merges(&forge, number);
    }
    let gate = Arc::new(Semaphore::new(0));
    forge.kit.stub.gate(&format!("{BASE}/pulls/2/merge"), &gate);
    let targets = (1..=3).map(|n| target(&forge, n)).collect();

    let late = async {
        while sent(&forge, 2).is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        forge.clock.advance(91);
        gate.add_permits(1);
    };
    let (response, ()) = tokio::join!(merge_many(&forge, targets), late);

    let got: Vec<String> = outcomes(&response).into_iter().map(|r| r.1).collect();
    assert_eq!(got, ["MERGED", "MERGED", "NOT_ATTEMPTED"], "{response}");
    assert_eq!(
        response["data"]["mergePullRequests"][2]["message"],
        "The hub stopped before it reached this one: 90 seconds passed. Nothing was changed."
    );
    assert!(sent(&forge, 3).is_empty(), "never started");
    assert_eq!(audits(&pool).await.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_a_bulk_merge_takes_1_to_20_pull_requests_each_once(pool: PgPool) {
    let forge = forge(&pool).await;
    let lists = [
        Vec::new(),
        (1..=21).map(|n| target(&forge, n)).collect(),
        vec![target(&forge, 1), target(&forge, 1)],
        vec![json!({ "repository": forge.repository, "number": 1, "expectedHeadSha": "x" })],
    ];
    for targets in lists {
        let response = merge_many(&forge, targets).await;
        assert_eq!(error(&response).1, "VALIDATION", "{response}");
        assert_eq!(
            response["errors"][0]["extensions"]["fields"][0]["field"],
            "pullRequests"
        );
    }
    assert_eq!(forge.live_reads(), 0, "nothing was read");
    assert_eq!(changes_sent(&forge), 0, "nothing was sent");
}
