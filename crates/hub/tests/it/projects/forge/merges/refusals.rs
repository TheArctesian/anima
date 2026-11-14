//! K-E1 Sections 13.3 and 13.6: a merge the rule refuses (its state, its head, its method, its
//! mergeability), or whose input is not one, sends nothing to GitHub and records nothing. The
//! words are fixed; a `VALIDATION` error names the field, never its value (LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{changes_sent, merge, methods, open};
use crate::{
    hub::error,
    projects::forge::{BASE, changes::audits, forge, pull},
};

const MARK: &str = "REFUSED-VALUE-4402";

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_each_pull_request_the_rule_refuses_sends_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    let mut draft = open(2, "draft", json!(true));
    draft["draft"] = json!(true);
    let cases: [(u64, Value, Value, &str); 8] = [
        (
            1,
            pull(1, "closed"),
            json!({}),
            "It cannot merge now: it is closed.",
        ),
        (2, draft, json!({}), "It cannot merge now: it is a draft."),
        (
            3,
            open(3, "dirty", json!(false)),
            json!({}),
            "It cannot merge now: it has a conflict.",
        ),
        (
            4,
            open(4, "blocked", json!(true)),
            json!({ "allowFailingChecks": true }),
            "It cannot merge now: a required check or review is missing.",
        ),
        (
            5,
            open(5, "behind", json!(true)),
            json!({}),
            "It cannot merge now: the base branch moved. Update the branch.",
        ),
        (
            6,
            open(6, "unstable", json!(true)),
            json!({}),
            "It cannot merge now: a check failed or still runs. Merge it alone with \
             allowFailingChecks.",
        ),
        (
            7,
            open(7, "clean", json!(true)),
            json!({ "expectedHeadSha": "f".repeat(40) }),
            "The pull request changed since you looked. Read it again.",
        ),
        (
            8,
            open(8, "clean", json!(true)),
            json!({ "method": "REBASE" }),
            "The repository does not allow that merge method.",
        ),
    ];
    for (number, read, extra, words) in cases {
        forge.ok(&format!("/pulls/{number}"), &read);
        let response = merge(&forge, number, &extra).await;
        assert_eq!(
            error(&response),
            (words.to_owned(), "CONFLICT".to_owned()),
            "{number}"
        );
    }
    assert_eq!(changes_sent(&forge), 0, "nothing");
    assert!(audits(&pool).await.is_empty(), "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_mergeability_github_has_not_worked_out_is_read_three_times_then_refused(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/9", &open(9, "unknown", Value::Null));

    let response = merge(&forge, 9, &json!({})).await;

    assert_eq!(
        error(&response),
        (
            "GitHub has not worked out whether it can merge. Try again.".to_owned(),
            "CONFLICT".to_owned()
        )
    );
    assert_eq!(
        forge.heard("/pulls/9").len(),
        3,
        "three reads, none from the cache"
    );
    assert_eq!(changes_sent(&forge), 0, "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_merge_methods_the_hub_cannot_see_refuse_the_merge(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("", &json!({ "id": 1, "full_name": "octo/widgets" }));
    forge.ok("/pulls/3", &open(3, "clean", json!(true)));

    let response = merge(&forge, 3, &json!({})).await;

    assert_eq!(
        error(&response),
        (
            "The hub cannot see which merge methods the repository allows. Give the GitHub App \
             Contents write."
                .to_owned(),
            "CONFLICT".to_owned()
        )
    );
    assert_eq!(changes_sent(&forge), 0, "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_bad_input_is_validation_naming_the_field_and_reads_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    let cases = [
        (
            json!({ "expectedHeadSha": format!("{MARK}abc") }),
            vec!["expectedHeadSha"],
        ),
        (json!({ "number": 0 }), vec!["number"]),
        (
            json!({ "number": -1, "expectedHeadSha": "" }),
            vec!["number", "expectedHeadSha"],
        ),
    ];
    for (extra, expected) in cases {
        let response = merge(&forge, 5, &extra).await;
        assert_eq!(error(&response).1, "VALIDATION", "{response}");
        let fields: Vec<&str> = response["errors"][0]["extensions"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["field"].as_str().unwrap())
            .collect();
        assert_eq!(fields, expected);
        assert!(!response.to_string().contains(MARK), "{response}");
    }
    let unknown = forge
        .kit
        .post(
            super::MERGE,
            json!({ "input": { "repository": "0190a000-0000-7000-8000-0000000000aa",
                "number": 5, "expectedHeadSha": "a".repeat(40), "method": "MERGE" } }),
        )
        .await;
    assert_eq!(error(&unknown).1, "NOT_FOUND", "{unknown}");
    let reads = forge.kit.stub.gets();
    assert!(
        reads
            .iter()
            .all(|path| !path.starts_with(&format!("{BASE}/pulls"))),
        "{reads:?}"
    );
}
