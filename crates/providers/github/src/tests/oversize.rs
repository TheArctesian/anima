//! A reply larger than the hub reads (SRC-006, SRC-007, PRO-003): for one repository it is that
//! repository's problem, read for its `pushed_at`; for the listing it stops the run.

use super::*;

const PUSHED: &str = "2026-11-10T17:01:58Z";

/// Three repositories, `me/a`, `me/b` and `me/c`, with ids 1, 2 and 3.
fn three() -> Vec<u8> {
    let repositories: Vec<Value> = ["me/a", "me/b", "me/c"]
        .iter()
        .zip(1..)
        .map(|(name, id)| repository(id, name, PUSHED))
        .collect();
    listing(3, &repositories)
}

/// The state after the listing of `page`, from `state`.
fn listed(state: &Value, page: &[u8]) -> Value {
    let state = begin(state);
    let request = next_request(&state).unwrap();
    let response = Response {
        status: 200,
        body: Body::Read(page),
    };
    normalise(&request, &response, &state).state
}

/// The answer to the next request of `state` with `body` and status 200.
fn answer(state: &Value, body: Body<'_>) -> (Request, Normalised<Record>) {
    let request = next_request(state).unwrap();
    let out = normalise(&request, &Response { status: 200, body }, state);
    (request, out)
}

/// Reads repository `me/a` whole: its tree and its deployments.
fn read_first(state: &Value) -> Value {
    let tree = empty_tree();
    let (_, out) = answer(state, Body::Read(&tree));
    answer(&out.state, Body::Read(b"[]")).1.state
}

#[test]
fn src_007_an_oversize_tree_is_the_repository_s_problem_and_the_run_moves_on() {
    let state = read_first(&listed(&json!({}), &three()));

    let (request, out) = answer(&state, Body::TooLarge);

    assert_eq!(request.path, "/repos/me/b/git/trees/main");
    assert_eq!(out.records, [], "no signs read for it this time");
    assert_eq!(
        out.problems,
        [Problem {
            code: "tree_too_large",
            subject: "me/b".to_owned(),
            message: "the repository's file list is larger than the hub reads".to_owned(),
        }]
    );
    assert_eq!(out.state["seen"]["2"], PUSHED, "read for its pushed_at");
    let next = next_request(&out.state).unwrap();
    assert_eq!(next.path, "/repos/me/c/git/trees/main");
}

#[test]
fn src_007_an_oversize_tree_is_read_again_only_after_a_new_push() {
    let state = read_first(&listed(&json!({}), &three()));
    let mut state = answer(&state, Body::TooLarge).1.state;
    state = read_first(&state);
    assert!(next_request(&state).is_none(), "the run ends");

    let again = listed(&state, &three());
    let queued: Vec<&Value> = again["pending"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pending| &pending["name"])
        .collect();
    assert_eq!(queued, Vec::<&Value>::new(), "nothing changed");

    let pushed = listing(1, &[repository(2, "me/b", "2026-11-12T09:00:00Z")]);
    let after_push = listed(&state, &pushed);
    let next = next_request(&after_push).unwrap();
    assert_eq!(next.path, "/repos/me/b/git/trees/main");
}

#[test]
fn src_007_an_oversize_deployments_reply_keeps_the_tree_and_moves_on() {
    let state = listed(&json!({}), &three());
    let tree = fixture("tree_tauri.json");
    let state = answer(&state, Body::Read(&tree)).1.state;

    let (request, out) = answer(&state, Body::TooLarge);

    assert_eq!(request.path, "/repos/me/a/deployments");
    assert_eq!(
        out.records,
        [],
        "no deployment signs replace the stored ones"
    );
    assert_eq!(out.problems[0].code, "deployments_too_large");
    assert_eq!(out.problems[0].subject, "me/a");
    assert_eq!(out.state["seen"]["1"], PUSHED);
    let next = next_request(&out.state).unwrap();
    assert_eq!(next.path, "/repos/me/b/git/trees/main");
}

#[test]
fn src_007_an_oversize_listing_stops_the_run_where_it_was() {
    let state = begin(&json!({}));

    let (_, out) = answer(&state, Body::TooLarge);

    assert_eq!(out.records, []);
    assert_eq!(
        out.state, state,
        "the slice does not move: the engine fails the run"
    );
    assert_eq!(out.problems[0].code, "listing_too_large");
    assert_eq!(out.problems[0].subject, "repositories");
}

#[test]
fn src_007_an_oversize_reply_to_an_unexpected_status_is_still_that_status() {
    let state = listed(&json!({}), &three());
    let request = next_request(&state).unwrap();
    let response = Response {
        status: 500,
        body: Body::TooLarge,
    };

    let out = normalise(&request, &response, &state);

    assert_eq!(out.state, state);
    assert_eq!(out.problems[0].code, "unexpected_status");
}

#[test]
fn pro_003_a_truncated_tree_is_read_for_its_pushed_at_and_its_signs_are_kept() {
    let run = run(
        &json!({}),
        &Replies {
            tree: (200, fixture("tree_truncated.json")),
            ..Replies::new(one_repository())
        },
    );

    assert_eq!(run.state["phase"], "done", "the run ends; it does not stop");
    assert_eq!(run.state["seen"][ANIMA], "2026-11-10T17:01:58Z");
    assert_eq!(
        signs_of(&run, ANIMA, Origin::Tree),
        tree_only(SignKind::WranglerConfig, "wrangler.jsonc")
    );
    let again = run_again(&run.state);
    assert!(
        again
            .iter()
            .all(|request| !request.path.contains("/git/trees/")),
        "{again:?}"
    );
}

/// The requests of a second run over the same listing.
fn run_again(state: &Value) -> Vec<Request> {
    run(state, &Replies::new(one_repository())).requests
}
