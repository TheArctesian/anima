//! SRC-008: a branch name never moves a request off the repository it names. A branch with a
//! `.` or `..` segment, from GitHub's reply or from a stored slice, is never read.

use super::*;

const HOSTILE: &str = "../../../../../app/installations";

fn no_dot_segment(run: &Run) {
    for request in &run.requests {
        assert!(
            !request.path.split('/').any(|s| s == "." || s == ".."),
            "{}",
            request.path
        );
        assert!(!request.path.contains("installations"), "{}", request.path);
    }
}

#[test]
fn src_008_a_default_branch_with_dot_segments_is_reported_and_not_read() {
    for branch in [HOSTILE, "..", ".", "release/../main", "a/./b"] {
        let mut entry = repository(7, "arctesian/odd", "2026-10-01T08:00:00Z");
        entry["default_branch"] = json!(branch);

        let run = run(&json!({}), &Replies::new(vec![listing(1, &[entry])]));

        assert!(
            run.records
                .iter()
                .any(|r| matches!(r, Record::Repository(r) if r.external_id == "7")),
            "the repository is still reported"
        );
        assert_eq!(run.requests.len(), 1, "only the listing: {branch}");
        assert_eq!(run.problems[0].code, "branch_unusable");
        assert!(
            !run.problems[0].message.contains(".."),
            "fixed words, never the branch"
        );
        no_dot_segment(&run);
    }
}

#[test]
fn src_008_a_stored_slice_with_a_hostile_branch_makes_no_such_request() {
    let pending = |id: &str, branch: &str| {
        json!({ "id": id, "name": "arctesian/odd", "branch": branch,
            "pushed_at": null, "step": "tree" })
    };
    let state = json!({
        "v": 1, "phase": "signs", "page": 1, "listed": [], "seen": {},
        "pending": [pending("7", HOSTILE), pending("8", "main")],
    });

    let run = run(&state, &Replies::new(Vec::new()));

    no_dot_segment(&run);
    assert_eq!(
        run.requests
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        [
            "/repos/arctesian/odd/git/trees/main",
            "/repos/arctesian/odd/deployments"
        ],
        "the hostile entry is dropped, the other is read"
    );
}

#[test]
fn src_008_a_dot_inside_a_branch_segment_is_still_read() {
    let mut entry = repository(9, "arctesian/odd", "2026-10-01T08:00:00Z");
    entry["default_branch"] = json!("release/v1.2");

    let run = run(&json!({}), &Replies::new(vec![listing(1, &[entry])]));

    assert!(
        run.requests
            .iter()
            .any(|r| r.path == "/repos/arctesian/odd/git/trees/release/v1.2")
    );
    assert_eq!(run.problems.len(), 0);
}
