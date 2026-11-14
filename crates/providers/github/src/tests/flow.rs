use source_exchange::Method;

use super::*;

fn repositories_in(run: &Run) -> Vec<&Repository> {
    run.records
        .iter()
        .filter_map(|record| match record {
            Record::Repository(repository) => Some(repository),
            _ => None,
        })
        .collect()
}

fn tree_requests(run: &Run) -> usize {
    run.requests
        .iter()
        .filter(|request| request.path.contains("/git/trees/"))
        .count()
}

#[test]
fn src_006_two_listing_pages_give_all_repositories_and_one_listed() {
    let first: Vec<Value> = (1..=100)
        .map(|id| repository(id, &format!("arctesian/repo-{id}"), "2026-10-01T08:00:00Z"))
        .collect();
    let last: Value =
        serde_json::from_slice(&fixture("installation_repositories_last_page.json")).unwrap();
    let replies = Replies::new(vec![
        listing(102, &first),
        serde_json::to_vec(&last).unwrap(),
    ]);

    let run = run(&json!({}), &replies);

    assert_eq!(repositories_in(&run).len(), 102);
    let listed: Vec<_> = run
        .records
        .iter()
        .filter_map(|record| match record {
            Record::Listed { external_ids } => Some(external_ids),
            _ => None,
        })
        .collect();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].len(), 102);
    assert_eq!(query(&run.requests[0], "page"), "1");
    assert_eq!(query(&run.requests[1], "page"), "2");
    assert_eq!(query(&run.requests[1], "per_page"), "100");
    assert_eq!(run.problems.len(), 0);
}

#[test]
fn src_006_a_listed_repository_keeps_every_field_github_sent() {
    let run = run(&json!({}), &Replies::new(one_repository()));

    let repository = repositories_in(&run)[0];
    assert_eq!(repository.external_id, ANIMA);
    assert_eq!(repository.full_name, "arctesian/anima");
    assert_eq!(repository.default_branch.as_deref(), Some("main"));
    assert!(repository.private && !repository.archived && !repository.fork);
    assert_eq!(repository.original["stargazers_count"], 0);
    assert_eq!(repository.original["owner"]["login"], "arctesian");
}

#[test]
fn prj_016_an_unchanged_pushed_at_makes_no_tree_request() {
    let replies = Replies::new(one_repository());
    let first = run(&json!({}), &replies);
    assert_eq!(tree_requests(&first), 1);

    let second = run(&first.state, &replies);

    assert_eq!(tree_requests(&second), 0);
    assert_eq!(second.requests.len(), 1, "only the listing");
    assert_eq!(
        repositories_in(&second).len(),
        1,
        "the repository is still reported"
    );
}

#[test]
fn prj_016_a_new_pushed_at_reads_the_signs_again() {
    let first = run(&json!({}), &Replies::new(one_repository()));
    let pushed = vec![listing(
        1,
        &[repository(
            612_345_678,
            "arctesian/anima",
            "2026-11-12T09:00:00Z",
        )],
    )];

    let second = run(&first.state, &Replies::new(pushed));

    assert_eq!(tree_requests(&second), 1);
}

#[test]
fn prj_016_a_rename_keeps_the_repository_id_and_follows_the_name() {
    let first = run(&json!({}), &Replies::new(one_repository()));
    let renamed = vec![listing(
        1,
        &[repository(
            612_345_678,
            "arctesian/anima-hub",
            "2026-11-10T17:01:58Z",
        )],
    )];

    let second = run(&first.state, &Replies::new(renamed));

    let repository = repositories_in(&second)[0];
    assert_eq!(repository.external_id, ANIMA);
    assert_eq!(repository.full_name, "arctesian/anima-hub");
    assert_eq!(tree_requests(&second), 0, "same push, so not read again");
}

#[test]
fn pro_003_a_repository_missing_from_a_later_listing_is_absent_from_listed() {
    let both = vec![listing(
        2,
        &[
            repository(1, "arctesian/a", "2026-10-01T08:00:00Z"),
            repository(2, "arctesian/b", "2026-10-01T08:00:00Z"),
        ],
    )];
    let first = run(&json!({}), &Replies::new(both));
    let only_a = vec![listing(
        1,
        &[repository(1, "arctesian/a", "2026-10-01T08:00:00Z")],
    )];

    let second = run(&first.state, &Replies::new(only_a));

    assert!(second.records.contains(&Record::Listed {
        external_ids: vec!["1".to_owned()]
    }));
    assert!(
        second.state["seen"].get("2").is_none(),
        "what was seen of it is dropped"
    );
}

#[test]
fn prj_016_a_repository_the_operator_left_out_is_forgotten_by_the_slice() {
    let first = run(&json!({}), &Replies::new(one_repository()));
    let pushed = [listing(
        1,
        &[repository(
            612_345_678,
            "arctesian/anima",
            "2026-11-12T09:00:00Z",
        )],
    )];
    let mut stepped = begin(&first.state);
    let request = next_request(&stepped).unwrap();
    let body = &pushed[0];
    stepped = normalise(&request, &Response { status: 200, body }, &stepped).state;
    assert!(next_request(&stepped).is_some(), "a tree request is queued");

    let forgotten = forget_repository(&stepped, ANIMA);

    assert!(
        next_request(&forgotten).is_none(),
        "no tree request is made for it"
    );
    assert!(
        forgotten["seen"].get(ANIMA).is_none(),
        "restoring reads its signs again"
    );
}

#[test]
fn src_006_a_status_with_no_rule_gives_a_problem_and_keeps_the_slice() {
    let state = begin(&json!({}));
    let request = next_request(&state).unwrap();

    let out = normalise(
        &request,
        &Response {
            status: 500,
            body: b"oops",
        },
        &state,
    );

    assert_eq!(out.records.len(), 0);
    assert_eq!(out.state, state);
    assert_eq!(out.problems.len(), 1);
    assert_eq!(out.problems[0].code, "unexpected_status");
    assert_eq!(out.problems[0].message, "GitHub answered with status 500");
}

#[test]
fn src_007_an_unreadable_listing_reports_no_repository_so_none_reads_as_gone() {
    let state = begin(&json!({}));
    let request = next_request(&state).unwrap();
    let broken = br#"{"total_count":1,"repositories":[{"id":1}]}"#;

    let out = normalise(
        &request,
        &Response {
            status: 200,
            body: broken,
        },
        &state,
    );

    assert_eq!(out.records.len(), 0);
    assert_eq!(out.state, state);
    assert_eq!(out.problems[0].code, "listing_unreadable");
}

#[test]
fn src_006_a_response_for_another_request_is_refused() {
    let state = begin(&json!({}));
    let other = Request::get(
        "/installation/repositories",
        &[("per_page", "100"), ("page", "9")],
    )
    .unwrap();

    let out = normalise(
        &other,
        &Response {
            status: 200,
            body: b"{}",
        },
        &state,
    );

    assert_eq!(out.records.len(), 0);
    assert_eq!(out.problems[0].code, "response_out_of_step");
}

#[test]
fn src_006_normalising_the_same_input_twice_gives_equal_output() {
    let state = begin(&json!({}));
    let request = next_request(&state).unwrap();
    let body = &one_repository()[0];
    let response = Response { status: 200, body };

    assert_eq!(
        normalise(&request, &response, &state),
        normalise(&request, &response, &state)
    );
    let replies = Replies::new(one_repository());
    let (a, b) = (run(&json!({}), &replies), run(&json!({}), &replies));
    assert_eq!(a.records, b.records);
    assert_eq!(a.state, b.state);
}

#[test]
fn src_006_no_request_is_anything_but_a_get_below_the_api_base() {
    let replies = Replies::new(one_repository());
    let run = run(&json!({}), &replies);

    assert!(run.requests.len() >= 3, "listing, tree and deployments");
    for request in &run.requests {
        assert_eq!(request.method, Method::Get);
        assert!(request.path.starts_with('/'), "{}", request.path);
        assert!(!request.path.contains("://") && !request.path.contains('@'));
    }
    assert!(DECLARATION.api_base.starts_with("https://"));
}

#[test]
fn prj_016_a_branch_with_unsafe_characters_is_encoded_in_the_tree_path() {
    let mut entry = repository(5, "arctesian/odd", "2026-10-01T08:00:00Z");
    entry["default_branch"] = json!("feature/a@b#c?d");
    let run = run(&json!({}), &Replies::new(vec![listing(1, &[entry])]));

    let tree = run
        .requests
        .iter()
        .find(|r| r.path.contains("/git/trees/"))
        .unwrap();
    assert_eq!(
        tree.path,
        "/repos/arctesian/odd/git/trees/feature/a%40b%23c%3Fd"
    );
    assert_eq!(query(tree, "recursive"), "1");
}

#[test]
fn prj_016_a_name_that_cannot_go_into_a_path_is_reported_and_not_read() {
    let run = run(
        &json!({}),
        &Replies::new(vec![listing(
            1,
            &[repository(6, "arctesian/a b", "2026-10-01T08:00:00Z")],
        )]),
    );

    assert_eq!(repositories_in(&run).len(), 1);
    assert_eq!(run.problems[0].code, "repository_name_unusable");
    assert_eq!(tree_requests(&run), 0);
}
