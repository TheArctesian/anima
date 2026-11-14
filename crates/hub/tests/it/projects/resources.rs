//! Signs give proposed resources found in the repository (PRO-003); a sign that goes removes
//! a found proposal but never a confirmed one, which only the operator makes (PRO-004).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list};
use super::kit;
use crate::hub::error;

const CONFIRM: &str = "mutation($id: ID!) { confirmProjectResource(id: $id) \
    { id kind state evidence { sign path } confirmedAt } }";

/// `(kind, state, evidence)` of each resource, sorted by kind.
fn shown(project: &Value) -> Vec<(String, String, Value)> {
    let mut all: Vec<(String, String, Value)> = project["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let text = |field: &str| r[field].as_str().unwrap().to_owned();
            (text("kind"), text("state"), r["evidence"].clone())
        })
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

fn found(kind: &str, evidence: Value) -> (String, String, Value) {
    (kind.to_owned(), "FOUND_IN_REPOSITORY".to_owned(), evidence)
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_003_each_sign_gives_a_resource_found_in_the_repository(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    let paths = ["wrangler.toml", "node_modules/x/Dockerfile", "README.md"];
    files(&kit.stub, "me/site", &paths, &["vercel[bot]"]);
    list(&kit.stub, &[Repo::new(1, "me/site")]);
    kit.run(id).await;
    let site = kit.project("site").await;
    assert_eq!(
        shown(&site),
        [
            found(
                "CLOUDFLARE",
                json!([{ "sign": "WRANGLER_CONFIG", "path": "wrangler.toml" }])
            ),
            found(
                "VERCEL",
                json!([{ "sign": "VERCEL_DEPLOYMENT", "path": null }])
            ),
        ]
    );
    assert!(site["resources"][0]["confirmedAt"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_004_a_sign_that_goes_removes_a_found_resource_but_not_a_confirmed_one(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    files(&kit.stub, "me/site", &["Dockerfile", "vercel.json"], &[]);
    list(&kit.stub, &[Repo::new(1, "me/site")]);
    kit.run(id).await;
    let site = kit.project("site").await;
    let server = site["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "SERVER")
        .unwrap()["id"]
        .clone();
    let confirmed = kit.post(CONFIRM, json!({ "id": server })).await;
    let resource = &confirmed["data"]["confirmProjectResource"];
    assert_eq!(resource["state"], "CONFIRMED", "{confirmed}");
    assert!(resource["confirmedAt"].is_string());

    // Both files go in a later push.
    files(&kit.stub, "me/site", &["README.md"], &[]);
    let mut pushed = Repo::new(1, "me/site");
    pushed.pushed_at = "2030-05-03T10:00:00Z".to_owned();
    list(&kit.stub, &[pushed]);
    kit.run(id).await;
    let site = kit.project("site").await;
    assert_eq!(
        shown(&site),
        [("SERVER".to_owned(), "CONFIRMED".to_owned(), json!([]))],
        "the found VERCEL went; the confirmed SERVER stays"
    );
    assert_eq!(site["resources"][0]["id"], server);
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_004_confirming_twice_is_conflict_and_an_unknown_id_is_not_found(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    files(&kit.stub, "me/site", &["compose.yaml"], &[]);
    list(&kit.stub, &[Repo::new(1, "me/site")]);
    kit.run(id).await;
    let server = kit.project("site").await["resources"][0]["id"].clone();
    kit.post(CONFIRM, json!({ "id": server })).await;
    let again = kit.post(CONFIRM, json!({ "id": server })).await;
    assert_eq!(error(&again).1, "CONFLICT", "{again}");
    let unknown = kit
        .post(
            CONFIRM,
            json!({ "id": "0190d6a4-0000-7000-8000-000000000000" }),
        )
        .await;
    assert_eq!(error(&unknown).1, "NOT_FOUND", "{unknown}");
    let audits: Vec<(String, String)> = sqlx::query_as(
        "SELECT action, actor_kind FROM events.audit_log WHERE target_type = 'project'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audits, [("accept".to_owned(), "user".to_owned())]);
}
