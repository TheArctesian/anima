use super::*;

fn replies_with(tree: &str, deployments: &str) -> Replies {
    let mut replies = Replies::new(one_repository());
    replies.tree = (200, fixture(tree));
    replies.deployments = (200, fixture(deployments));
    replies
}

#[test]
fn pro_003_a_nested_tauri_conf_gives_the_tauri_sign() {
    let run = run(
        &json!({}),
        &replies_with("tree_tauri.json", "deployments_other.json"),
    );

    let signs = signs_of(&run, ANIMA, Origin::Tree);
    assert!(signs.contains(&Sign {
        kind: SignKind::TauriConf,
        path: Some("apps/x/src-tauri/tauri.conf.json".to_owned())
    }));
    assert!(signs.contains(&Sign {
        kind: SignKind::Dockerfile,
        path: Some("Dockerfile".to_owned())
    }));
}

#[test]
fn pro_003_a_file_under_node_modules_gives_no_sign() {
    let run = run(
        &json!({}),
        &replies_with("tree_node_modules.json", "deployments_other.json"),
    );

    assert_eq!(signs_of(&run, ANIMA, Origin::Tree).len(), 0);
}

#[test]
fn pro_003_a_vercel_bot_deployment_gives_the_vercel_deployment_sign() {
    let run = run(
        &json!({}),
        &replies_with("tree_tauri.json", "deployments_vercel.json"),
    );

    assert_eq!(
        signs_of(&run, ANIMA, Origin::Deployments),
        [Sign {
            kind: SignKind::VercelDeployment,
            path: None
        }]
    );
}

#[test]
fn pro_003_deployments_by_other_creators_give_no_sign() {
    let run = run(
        &json!({}),
        &replies_with("tree_tauri.json", "deployments_other.json"),
    );

    assert_eq!(signs_of(&run, ANIMA, Origin::Deployments).len(), 0);
}

#[test]
fn pro_003_an_empty_repository_status_gives_empty_signs() {
    let mut replies = Replies::new(one_repository());
    replies.tree = (409, fixture("tree_empty_repository.json"));

    let run = run(&json!({}), &replies);

    assert_eq!(signs_of(&run, ANIMA, Origin::Tree).len(), 0);
    assert_eq!(
        run.problems.len(),
        0,
        "an empty repository is not a problem"
    );
    assert_eq!(run.state["phase"], "done");
}

#[test]
fn pro_003_a_truncated_tree_gives_a_problem_and_the_signs_it_has() {
    let run = run(
        &json!({}),
        &replies_with("tree_truncated.json", "deployments_other.json"),
    );

    assert_eq!(run.problems.len(), 1);
    assert_eq!(run.problems[0].code, "tree_truncated");
    assert_eq!(run.problems[0].subject, "arctesian/anima");
    assert_eq!(
        signs_of(&run, ANIMA, Origin::Tree),
        tree_only(SignKind::WranglerConfig, "wrangler.jsonc")
    );
}

#[test]
fn pro_003_missing_deployments_permission_keeps_the_files_and_reports_it() {
    let mut replies = replies_with("tree_tauri.json", "deployments_other.json");
    replies.deployments = (
        403,
        br#"{"message":"Resource not accessible by integration"}"#.to_vec(),
    );

    let run = run(&json!({}), &replies);

    assert_eq!(run.problems[0].code, "deployments_unavailable");
    assert_ne!(signs_of(&run, ANIMA, Origin::Tree).len(), 0);
    assert_eq!(run.state["phase"], "done");
}

#[test]
fn src_007_an_interrupted_signs_phase_resumes_where_it_stopped() {
    let tree = fixture("tree_tauri.json");
    let state = begin(&json!({}));
    let list = next_request(&state).unwrap();
    let page = &one_repository()[0];
    let state = normalise(
        &list,
        &Response {
            status: 200,
            body: page,
        },
        &state,
    )
    .state;
    let tree_request = next_request(&state).unwrap();
    assert!(tree_request.path.contains("/git/trees/"));
    // The tree page is stored; the run dies before the deployments request.
    let state = normalise(
        &tree_request,
        &Response {
            status: 200,
            body: &tree,
        },
        &state,
    )
    .state;

    let resumed = begin(&state);

    assert_eq!(resumed, state, "begin keeps an unfinished phase");
    let next = next_request(&resumed).unwrap();
    assert_eq!(next.path, "/repos/arctesian/anima/deployments");
    assert_eq!(query(&next, "per_page"), "30");
    let out = normalise(
        &next,
        &Response {
            status: 200,
            body: b"[]",
        },
        &resumed,
    );
    assert_eq!(out.state["phase"], "done");
    assert!(next_request(&out.state).is_none());
}

#[test]
fn src_007_a_run_that_ended_cleanly_starts_a_new_listing() {
    let done = run(&json!({}), &Replies::new(one_repository())).state;
    assert_eq!(done["phase"], "done");

    let started = begin(&done);

    assert_eq!(started["phase"], "list");
    assert_eq!(started["page"], 1);
    assert!(
        started["seen"].get(ANIMA).is_some(),
        "what was seen is kept"
    );
}

#[test]
fn src_004_a_slice_of_another_version_starts_from_nothing() {
    let started = begin(&json!({ "v": 99, "phase": "signs", "pending": [{"id": "1"}] }));

    assert_eq!(started["v"], 1);
    assert_eq!(started["phase"], "list");
    assert_eq!(started["pending"], json!([]));
}
