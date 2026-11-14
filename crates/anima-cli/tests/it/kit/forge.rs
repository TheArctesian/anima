//! What the hub says of a repository on GitHub, as the scripted hub answers it: one repository,
//! one project, and the K1 rows of pull requests, issues, comments, workflows and runs.

use serde_json::{Value, json};

use super::fake_hub::{FakeHub, data};

pub const REPOSITORY: &str = "0190a000-0000-7000-9002-000000000001";
pub const FULL_NAME: &str = "acme/anima";
pub const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// Answers `ProjectList` with one project that holds `acme/anima`, so a name resolves to
/// [`REPOSITORY`].
pub fn project(hub: &FakeHub) {
    let repository = json!({
        "id": REPOSITORY, "fullName": FULL_NAME, "url": "https://github.com/acme/anima",
        "defaultBranch": "main", "private": true, "archived": false, "fork": false,
        "description": null, "pushedAt": null, "leftOut": false, "gone": false,
    });
    let project = json!({
        "id": "p-1", "slug": "anima", "name": "Anima", "subjectKey": "k",
        "repositories": [repository], "resources": [], "syncedAt": null,
    });
    hub.answer(
        "ProjectList",
        data(
            json!({ "projects": { "edges": [{ "cursor": "c", "node": project }],
                                    "pageInfo": { "hasNextPage": false, "endCursor": null } } }),
        ),
    );
}

pub fn read() -> Value {
    json!({ "at": "2026-11-14T09:00:00Z", "stale": false, "problem": null })
}

/// A connection of `nodes` under `root`, with a next page when `next` is a cursor.
pub fn connection(root: &str, nodes: &[Value], next: Option<&str>) -> Value {
    let edges: Vec<Value> = nodes
        .iter()
        .map(|node| json!({ "cursor": "c1", "node": node }))
        .collect();
    json!({ root: { "edges": edges, "read": read(), "pageInfo": {
        "hasNextPage": next.is_some(), "endCursor": next } } })
}

pub fn pull_request(number: i64, title: &str, body: Option<&str>) -> Value {
    json!({
        "repository": REPOSITORY, "number": number, "title": title, "state": "OPEN",
        "draft": false, "authorLogin": "octo", "headBranch": "fix-it", "headSha": SHA,
        "baseBranch": "main", "fromFork": false, "headRepository": FULL_NAME,
        "labels": ["bug"], "comments": 2, "createdAt": "2026-11-10T08:00:00Z",
        "updatedAt": "2026-11-12T08:00:00Z", "mergedAt": null,
        "url": format!("https://github.com/acme/anima/pull/{number}"),
        "body": body, "bodyCut": false,
        "mergeability": { "state": "CLEAN", "canMerge": true },
        "checks": { "sha": SHA, "state": "PASSED", "passed": 3, "failed": 0, "pending": 0,
                    "other": 0, "failing": [], "partial": false },
    })
}

pub fn issue(number: i64, title: &str, body: Option<&str>) -> Value {
    json!({
        "repository": REPOSITORY, "number": number, "title": title, "open": true,
        "closeReason": null, "authorLogin": "octo", "labels": ["help wanted"],
        "assignees": ["maint"], "comments": 1, "createdAt": "2026-11-10T08:00:00Z",
        "updatedAt": "2026-11-12T08:00:00Z", "closedAt": null,
        "url": format!("https://github.com/acme/anima/issues/{number}"),
        "body": body, "bodyCut": false,
    })
}

pub fn comment(id: &str, body: &str) -> Value {
    json!({
        "id": id, "authorLogin": "octo", "body": body, "bodyCut": false,
        "createdAt": "2026-11-11T08:00:00Z", "updatedAt": "2026-11-11T08:00:00Z",
        "url": format!("https://github.com/acme/anima/issues/7#issuecomment-{id}"),
    })
}

pub fn workflow() -> Value {
    json!({ "id": "42", "name": "CI", "path": ".github/workflows/ci.yml", "active": true,
            "url": "https://github.com/acme/anima/actions/workflows/ci.yml" })
}

pub fn run(id: &str, title: &str) -> Value {
    json!({
        "repository": REPOSITORY, "id": id, "workflowId": "42", "name": "CI", "runNumber": 9,
        "attempt": 1, "event": "push", "status": "COMPLETED", "conclusion": "FAILURE",
        "branch": "main", "headSha": SHA, "commitTitle": title, "actorLogin": "octo",
        "fromFork": false, "createdAt": "2026-11-12T08:00:00Z",
        "updatedAt": "2026-11-12T08:05:00Z", "startedAt": "2026-11-12T08:01:00Z",
        "url": format!("https://github.com/acme/anima/actions/runs/{id}"),
        "rerunnable": true, "failedJobsRerunnable": true, "cancellable": false,
        "jobs": { "total": 4, "succeeded": 3, "failed": 1, "running": 0, "queued": 0,
                  "other": 0, "failing": ["test (linux)"], "partial": false },
    })
}

pub fn commit(sha: &str, title: &str) -> Value {
    json!({
        "repository": REPOSITORY, "sha": sha, "title": title, "message": title,
        "authorName": "Octo Cat", "authorLogin": "octo", "committedAt": "2026-11-12T08:00:00Z",
        "parents": [SHA], "url": format!("https://github.com/acme/anima/commit/{sha}"),
        "checks": { "sha": sha, "state": "FAILED", "passed": 2, "failed": 1, "pending": 0,
                    "other": 0, "failing": ["test (linux)"], "partial": false },
    })
}

pub fn branch(name: &str, default: bool) -> Value {
    json!({ "name": name, "sha": SHA, "protected": default, "default": default })
}

pub fn repository_release(tag: &str) -> Value {
    json!({
        "id": "9001", "tag": tag, "name": "Spring", "draft": false, "prerelease": true,
        "authorLogin": "octo", "createdAt": "2026-11-01T08:00:00Z",
        "publishedAt": "2026-11-02T08:00:00Z",
        "url": format!("https://github.com/acme/anima/releases/tag/{tag}"),
    })
}

pub fn code_entry(name: &str, kind: &str, size: i64) -> Value {
    json!({ "name": name, "path": format!("src/{name}"), "kind": kind, "size": size,
            "sha": SHA })
}

pub fn code_directory(entries: &[Value], partial: bool) -> Value {
    json!({ "codeDirectory": {
        "repository": REPOSITORY, "ref": "main", "path": "src", "entries": entries,
        "partial": partial, "read": read() } })
}

/// A `CodeFile`: `content` is `TEXT`, `BINARY` or `TOO_LARGE`; `text` only for `TEXT`.
pub fn code_file(path: &str, content: &str, text: Option<&str>) -> Value {
    json!({ "codeFile": {
        "repository": REPOSITORY, "ref": "main", "path": path, "name": "notes.txt", "sha": SHA,
        "size": 12, "content": content, "text": text,
        "url": format!("https://github.com/acme/anima/blob/main/{path}"),
        "readAt": "2026-11-14T09:00:00Z" } })
}
