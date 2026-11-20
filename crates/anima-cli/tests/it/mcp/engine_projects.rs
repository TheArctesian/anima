//! The tools of the `projects` set against the scripted hub (MCP-008, MCP-017).

use serde_json::{Value, json};

use super::engine::{sent, structured, writing};
use super::session::{Desk, LOGIN_TOKEN, hub};
use crate::kit::fake_hub::{FakeHub, data};

const REPOSITORY: &str = "0190a000-0000-7000-9002-000000000001";

fn repository(description: &str, left_out: bool) -> Value {
    json!({ "id": REPOSITORY, "fullName": "arctesian/anima", "url": "https://github.com/a/b",
            "defaultBranch": "main", "private": true, "archived": false, "fork": false,
            "description": description, "pushedAt": null, "leftOut": left_out, "gone": false })
}

fn resource(state: &str) -> Value {
    json!({ "id": "res-1", "kind": "VERCEL", "state": state,
            "evidence": [{ "sign": "VERCEL_JSON", "path": "vercel.json" }],
            "foundAt": "2026-11-12T10:00:00Z", "confirmedAt": null })
}

fn project(description: &str) -> Value {
    json!({ "id": "p-1", "slug": "anima", "name": "Anima", "subjectKey": "k",
            "repositories": [repository(description, false)],
            "resources": [resource("FOUND_IN_REPOSITORY")], "syncedAt": null })
}

fn projects_answer(hub: &FakeHub, description: &str) {
    hub.answer(
        "ProjectList",
        data(
            json!({ "projects": { "edges": [{ "cursor": "c1", "node": project(description) }],
                                   "pageInfo": { "hasNextPage": false, "endCursor": "c1" } } }),
        ),
    );
}

#[tokio::test]
async fn mcp_017_projects_search_returns_a_page_of_projects() {
    let hub = hub().await;
    projects_answer(&hub, "A device manager");
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session
        .call(
            "anima_projects_search",
            json!({ "leftOut": true, "limit": 5 }),
        )
        .await;
    let result = structured(&answer);
    assert_eq!(result["projects"][0]["slug"], "anima");
    assert_eq!(result["nextCursor"], Value::Null);
    let asked = &sent(&hub, "ProjectList")[0];
    assert_eq!(
        (&asked["leftOut"], &asked["first"]),
        (&json!(true), &json!(5))
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_project_get_returns_one_project_or_says_there_is_none() {
    let hub = hub().await;
    hub.answer("ProjectGet", data(json!({ "project": project("x") })));
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session
        .call("anima_project_get", json!({ "slug": "anima" }))
        .await;
    assert_eq!(
        structured(&answer)["project"]["resources"][0]["kind"],
        "VERCEL"
    );
    session.finish().await;

    let hub = self::hub().await;
    hub.answer("ProjectGet", data(json!({ "project": null })));
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let missing = session
        .call("anima_project_get", json!({ "slug": "nope" }))
        .await;
    assert_eq!(missing["isError"], true, "{missing}");
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_repository_set_left_out_leaves_out_and_restores_by_full_name() {
    let hub = hub().await;
    projects_answer(&hub, "x");
    let left = json!({ "id": "p-1", "slug": "anima", "name": "Anima", "subjectKey": "k",
                       "repositories": [repository("x", true)], "syncedAt": null });
    hub.answer(
        "RepositoryLeaveOut",
        data(json!({ "leaveOutRepository": left })),
    );
    hub.answer(
        "RepositoryRestore",
        data(json!({ "restoreRepository": project("x") })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let out = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": "arctesian/anima", "leftOut": true }),
        )
        .await;
    assert_eq!(
        structured(&out)["project"]["repositories"][0]["leftOut"],
        true
    );
    let back = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": REPOSITORY, "leftOut": false }),
        )
        .await;
    assert_eq!(structured(&back)["project"]["slug"], "anima");
    assert_eq!(sent(&hub, "RepositoryLeaveOut")[0]["id"], REPOSITORY);
    assert_eq!(sent(&hub, "RepositoryRestore")[0]["id"], REPOSITORY);
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_project_resource_confirm_confirms_the_resources_of_one_kind() {
    let hub = hub().await;
    hub.answer("ProjectGet", data(json!({ "project": project("x") })));
    let mut confirmed = resource("CONFIRMED");
    confirmed["confirmedAt"] = json!("2026-11-13T09:00:00Z");
    hub.answer(
        "ResourceConfirm",
        data(json!({ "confirmProjectResource": confirmed })),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_project_resource_confirm",
            json!({ "slug": "anima", "kind": "VERCEL" }),
        )
        .await;
    assert_eq!(structured(&answer)["resources"][0]["state"], "CONFIRMED");
    assert_eq!(sent(&hub, "ResourceConfirm")[0]["id"], "res-1");
    let none = session
        .call(
            "anima_project_resource_confirm",
            json!({ "slug": "anima", "kind": "SERVER" }),
        )
        .await;
    assert_eq!(none["isError"], true, "{none}");
    session.finish().await;
}

#[tokio::test]
async fn prj_016_set_left_out_refuses_a_name_two_repositories_hold_and_sends_nothing() {
    let hub = hub().await;
    let mut stale = repository("x", true);
    stale["id"] = json!("0190a000-0000-7000-9002-000000000009");
    let mut both = project("x");
    both["repositories"] = json!([stale, repository("x", false)]);
    hub.answer(
        "ProjectList",
        data(
            json!({ "projects": { "edges": [{ "cursor": "c1", "node": both }],
                                   "pageInfo": { "hasNextPage": false, "endCursor": "c1" } } }),
        ),
    );
    let mut session = writing(&hub, &Desk::new()).await;
    let answer = session
        .call(
            "anima_repository_set_left_out",
            json!({ "repository": "arctesian/anima", "leftOut": true }),
        )
        .await;
    assert_eq!(answer["isError"], true, "{answer}");
    let text = answer.to_string();
    assert!(
        text.contains("0190a000-0000-7000-9002-000000000009"),
        "{text}"
    );
    assert!(text.contains(REPOSITORY), "{text}");
    assert_eq!(sent(&hub, "RepositoryLeaveOut"), Vec::<Value>::new());
    session.finish().await;
}

#[tokio::test]
async fn mcp_008_a_repository_description_comes_back_cleaned_and_cut() {
    let hub = hub().await;
    let hostile = format!("ok\u{1b}[2J\u{7}line\nbreak {}", "x".repeat(2_000));
    projects_answer(&hub, &hostile);
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let answer = session.call("anima_projects_search", json!({})).await;
    let shown = structured(&answer)["projects"][0]["repositories"][0]["description"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(shown.starts_with("ok [2J line break "), "{shown}");
    assert!(!shown.chars().any(char::is_control), "{shown:?}");
    assert!(shown.contains("…[cut: "), "{shown}");
    assert!(shown.chars().count() < 600, "{}", shown.chars().count());
    session.finish().await;
}
