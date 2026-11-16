//! PRJ-032: `ProjectRepository.access` names why a repository cannot be read, and a list root
//! on a repository that is not READY answers `CONFLICT` without asking GitHub.

use serde_json::json;
use sqlx::PgPool;

use super::{Forge, forge};
use crate::hub::error;

const ACCESS: &str = "{ project(slug: \"widgets\") { repositories { access } } }";
const PULLS: &str =
    "query($repository: ID!) { pullRequests(repository: $repository) { edges { cursor } } }";
const RUNS: &str =
    "query($repository: ID!) { workflowRuns(repository: $repository) { edges { cursor } } }";

async fn access(forge: &Forge) -> String {
    let response = forge.kit.post(ACCESS, json!({})).await;
    response["data"]["project"]["repositories"][0]["access"]
        .as_str()
        .unwrap_or_else(|| panic!("no access: {response}"))
        .to_owned()
}

async fn run_sql(forge: &Forge, sql: &str) {
    sqlx::query(sql).execute(&forge.kit.pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_each_access_is_shown_and_a_list_is_refused_without_asking_github(pool: PgPool) {
    let forge = forge(&pool).await;
    assert_eq!(access(&forge).await, "READY");

    let cases = [
        (
            "UPDATE sources.accounts SET paused_at = now()",
            "UPDATE sources.accounts SET paused_at = NULL",
            "PAUSED",
            "Its account is paused. Resume it to read the repository.",
        ),
        (
            "UPDATE sources.accounts SET last_error_kind = 'auth'",
            "UPDATE sources.accounts SET last_error_kind = NULL",
            "NEEDS_RECONNECT",
            "GitHub refused the key of its account. Connect the account again.",
        ),
        (
            "UPDATE projects.repositories SET gone_at = now()",
            "UPDATE projects.repositories SET gone_at = NULL",
            "GONE",
            "The GitHub connection no longer lists this repository.",
        ),
        (
            "UPDATE projects.repositories SET left_out_at = now()",
            "UPDATE projects.repositories SET left_out_at = NULL",
            "LEFT_OUT",
            "This repository is left out. Restore it to read it.",
        ),
        (
            "UPDATE projects.repositories SET account_id = NULL",
            "SELECT 1",
            "NOT_CONNECTED",
            "This repository is not connected: no source account holds it.",
        ),
    ];
    for (set, undo, expected, words) in cases {
        run_sql(&forge, set).await;
        assert_eq!(access(&forge).await, expected);
        for query in [PULLS, RUNS] {
            let response = forge.ask(query, json!({})).await;
            assert_eq!(
                error(&response),
                (words.to_owned(), "CONFLICT".to_owned()),
                "{expected}"
            );
        }
        run_sql(&forge, undo).await;
    }
    assert_eq!(forge.live_reads(), 0, "GitHub was never asked");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_an_unknown_or_bad_repository_id_is_refused(pool: PgPool) {
    let forge = forge(&pool).await;
    let unknown = forge
        .kit
        .post(
            PULLS,
            json!({ "repository": "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d" }),
        )
        .await;
    assert_eq!(
        error(&unknown),
        (
            "No repository has this ID.".to_owned(),
            "NOT_FOUND".to_owned()
        )
    );
    let bad = forge
        .kit
        .post(PULLS, json!({ "repository": "octo/widgets" }))
        .await;
    assert_eq!(bad["errors"][0]["extensions"]["code"], "VALIDATION");
}
