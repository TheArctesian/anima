//! `anima project` against the scripted hub (PRJ-002, PRJ-016, PRO-004).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-project-tests")
}

fn repository(full_name: &str, left_out: bool) -> Value {
    json!({
        "id": format!("repo-{full_name}"), "fullName": full_name,
        "url": format!("https://github.com/{full_name}"), "defaultBranch": "main",
        "private": true, "archived": false, "fork": false, "description": null,
        "pushedAt": "2026-11-10T08:00:00Z", "leftOut": left_out, "gone": false,
    })
}

fn resource(id: &str, kind: &str, state: &str) -> Value {
    json!({
        "id": id, "kind": kind, "state": state,
        "evidence": [{ "sign": "WRANGLER_CONFIG", "path": "wrangler.toml" }],
        "foundAt": "2026-11-10T08:00:00Z",
        "confirmedAt": if state == "CONFIRMED" { json!("2026-11-11T08:00:00Z") } else { Value::Null },
    })
}

fn project(left_out: bool) -> Value {
    json!({
        "id": "project-1", "slug": "anima", "name": "Anima", "subjectKey": "acme/anima",
        "repositories": [repository("acme/anima", left_out)],
        "resources": [resource("res-1", "CLOUDFLARE", "FOUND_IN_REPOSITORY")],
        "syncedAt": "2026-11-12T08:00:00Z",
    })
}

fn connection(projects: &[Value]) -> Value {
    let edges: Vec<Value> = projects
        .iter()
        .map(|node| json!({ "cursor": "c1", "node": node }))
        .collect();
    json!({ "projects": { "edges": edges, "pageInfo": { "hasNextPage": false, "endCursor": null } } })
}

#[tokio::test]
async fn prj_016_project_list_and_get_print_the_k1_value_with_json() {
    let hub = FakeHub::start().await;
    hub.answer("ProjectList", data(connection(&[project(false)])));
    hub.answer("ProjectGet", data(json!({ "project": project(false) })));
    let run = signed_in()
        .run(&["project", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"][0]["node"], project(false));
    assert_eq!(hub.seen()[0].variables["leftOut"], false);

    let run = signed_in()
        .run(&["project", "list", "--left-out", "--hub", &hub.origin])
        .await;
    assert!(
        run.stdout.contains("anima") && run.stdout.contains("Anima"),
        "{run:?}"
    );
    assert_eq!(hub.seen()[1].variables["leftOut"], true);

    let run = signed_in()
        .run(&["project", "get", "anima", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), project(false));
    assert_eq!(hub.seen()[2].variables, json!({ "slug": "anima" }));
}

#[tokio::test]
async fn prj_016_project_get_for_an_unknown_slug_exits_3() {
    let hub = FakeHub::start().await;
    hub.answer("ProjectGet", data(json!({ "project": null })));
    let run = signed_in()
        .run(&["project", "get", "nope", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn prj_002_leave_out_and_restore_find_the_repository_by_full_name() {
    let hub = FakeHub::start().await;
    hub.answer("ProjectList", data(connection(&[project(false)])));
    let out = json!({ "leaveOutRepository": project(true) });
    hub.answer("RepositoryLeaveOut", data(out));
    let run = signed_in()
        .run(&[
            "project",
            "leave-out",
            "acme/anima",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["repositories"][0]["leftOut"], true);
    let seen = hub.seen();
    assert_eq!(
        seen.last().expect("sent").variables["id"],
        "repo-acme/anima"
    );

    let hub = FakeHub::start().await;
    hub.answer("ProjectList", data(connection(&[project(true)])));
    hub.answer(
        "RepositoryRestore",
        data(json!({ "restoreRepository": project(false) })),
    );
    let run = signed_in()
        .run(&[
            "project",
            "restore",
            "acme/anima",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["repositories"][0]["leftOut"], false);

    let run = signed_in()
        .run(&["project", "restore", "acme/unknown", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn pro_004_resource_confirm_confirms_the_resource_of_that_kind() {
    let hub = FakeHub::start().await;
    hub.answer("ProjectGet", data(json!({ "project": project(false) })));
    let confirmed = resource("res-1", "CLOUDFLARE", "CONFIRMED");
    hub.answer(
        "ResourceConfirm",
        data(json!({ "confirmProjectResource": confirmed })),
    );
    let run = signed_in()
        .run(&[
            "project",
            "resource",
            "confirm",
            "anima",
            "cloudflare",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), json!([confirmed]));
    assert_eq!(hub.seen().last().expect("sent").variables["id"], "res-1");

    let run = signed_in()
        .run(&[
            "project",
            "resource",
            "confirm",
            "anima",
            "vercel",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 3, "no vercel resource: {run:?}");
}

/// A left-out or gone repository keeps its old name, which another repository can take later.
/// The project lists both; the scripted hub answers both lists with them.
fn stale_and_new() -> Value {
    let mut old = repository("acme/anima", true);
    old["id"] = json!("repo-old");
    old["gone"] = json!(true);
    let mut new = repository("acme/anima", false);
    new["id"] = json!("repo-new");
    let mut both = project(false);
    both["repositories"] = json!([old, new]);
    connection(&[both])
}

#[tokio::test]
async fn prj_016_a_name_two_repositories_hold_is_refused_with_their_ids() {
    let hub = FakeHub::start().await;
    hub.answer("ProjectList", data(stale_and_new()));
    for command in ["leave-out", "restore"] {
        let run = signed_in()
            .run(&[
                "project",
                command,
                "acme/anima",
                "--json",
                "--hub",
                &hub.origin,
            ])
            .await;
        assert_eq!(run.code, 2, "{run:?}");
        let error = run.error();
        let message = error["message"].as_str().unwrap_or_default();
        for shown in [
            "repo-old (left out, gone)",
            "repo-new (included)",
            "Give the ID",
        ] {
            assert!(message.contains(shown), "{message}");
        }
    }
    let sent: Vec<String> = hub.seen().iter().map(|s| s.operation.clone()).collect();
    assert!(
        sent.iter().all(|operation| operation == "ProjectList"),
        "nothing changed: {sent:?}"
    );
}

#[tokio::test]
async fn pro_004_resource_confirm_shows_the_hubs_conflict_for_a_confirmed_resource() {
    let hub = FakeHub::start().await;
    let mut done = project(false);
    done["resources"] = json!([resource("res-1", "CLOUDFLARE", "CONFIRMED")]);
    hub.answer("ProjectGet", data(json!({ "project": done })));
    hub.answer("ResourceConfirm", refusal("CONFLICT", "already confirmed"));
    let run = signed_in()
        .run(&[
            "project",
            "resource",
            "confirm",
            "anima",
            "cloudflare",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
}
