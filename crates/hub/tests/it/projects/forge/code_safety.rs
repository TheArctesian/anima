//! K-E1 Section 13.8, D-23, LOG-007: what keeps the code browser safe. A hostile path or ref
//! is refused before anything is sent; names reach GitHub encoded; a directory is kept and a
//! file never, even when a directory read finds one; a file is closed to every API key; and neither a file's content nor its path
//! reaches a log line or an error message.

use anima_hub::fleet::events::process_log;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    code::{DIRECTORY, FILE, file},
    forge, sha,
    tokens::key,
};
use crate::{hub::error, logs::Capture, sources::set_projects};

const COMMITS: &str = "query($repository: ID!, $ref: String) { commits(repository: $repository, ref: $ref) { edges { cursor } } }";

/// The paths K-E1 Section 13.8 refuses.
fn hostile_paths() -> Vec<String> {
    let mut paths: Vec<String> = [
        "..",
        "a/../b",
        "./a",
        "a/./b",
        "/etc/passwd",
        "a//b",
        "a/",
        "a\\b",
        "a\u{0}b",
        "a\nb",
        "a\u{7f}b",
    ]
    .map(str::to_owned)
    .to_vec();
    paths.push("x".repeat(1_025));
    paths.push("s/".repeat(40) + "s");
    paths
}

fn hostile_refs() -> Vec<String> {
    let mut refs: Vec<String> = ["", "-x", "a..b", "a b", "a\nb", "a\u{0}b", "\t"]
        .map(str::to_owned)
        .to_vec();
    refs.push("r".repeat(256));
    refs
}

/// The one field a `VALIDATION` error names.
fn field(response: &Value) -> String {
    assert_eq!(error(response).1, "VALIDATION", "{response}");
    response["errors"][0]["extensions"]["fields"][0]["field"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_hostile_path_or_ref_is_refused_and_nothing_is_sent(pool: PgPool) {
    let forge = forge(&pool).await;
    for path in hostile_paths() {
        for query in [DIRECTORY, FILE] {
            let response = forge.ask(query, json!({ "path": path })).await;
            assert_eq!(field(&response), "path", "{path:?}");
        }
    }
    let empty = forge.ask(FILE, json!({ "path": "" })).await;
    assert_eq!(field(&empty), "path");
    for git_ref in hostile_refs() {
        for query in [DIRECTORY, FILE, COMMITS] {
            let variables = json!({ "path": "a.txt", "ref": git_ref });
            let response = forge.ask(query, variables).await;
            assert_eq!(field(&response), "ref", "{git_ref:?}");
        }
    }
    assert_eq!(forge.live_reads(), 0, "nothing reached GitHub");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_names_reach_github_encoded_one_segment_at_a_time(pool: PgPool) {
    let forge = forge(&pool).await;
    let encoded = "/contents/my%20dir/a%23b%3Fc/caf%C3%A9%20%E2%98%83.md/%252e";
    forge.ok(
        encoded,
        &file("my dir/a#b?c/caf\u{e9} \u{2603}.md/%2e", b"ok"),
    );
    let path = "my dir/a#b?c/caf\u{e9} \u{2603}.md/%2e";
    let response = forge
        .ask(FILE, json!({ "path": path, "ref": "feature/x#y?z" }))
        .await;
    assert_eq!(response["data"]["codeFile"]["text"], "ok", "{response}");
    assert_eq!(
        forge.heard(encoded),
        [format!(
            "/repos/octo/widgets{encoded}?ref=feature%2Fx%23y%3Fz"
        )]
    );
    assert_eq!(
        response["data"]["codeFile"]["url"],
        "https://github.com/octo/widgets/blob/feature/x%23y%3Fz/my%20dir/a%23b%3Fc/\
         caf%C3%A9%20%E2%98%83.md/%252e"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_directory_is_kept_and_a_file_never(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/contents/src", &json!([]));
    forge.ok("/contents/a.txt", &file("a.txt", b"a"));
    for _ in 0..2 {
        forge.ask(DIRECTORY, json!({ "path": "src" })).await;
        forge.ask(FILE, json!({ "path": "a.txt" })).await;
    }
    assert_eq!(forge.heard("/contents/src").len(), 1, "a directory is kept");
    assert_eq!(
        forge.heard("/contents/a.txt").len(),
        2,
        "a file is never kept"
    );

    forge.clock.advance(61);
    forge.ask(DIRECTORY, json!({ "path": "src" })).await;
    assert_eq!(
        forge.heard("/contents/src").len(),
        2,
        "a branch's directory: 60 s"
    );
}

/// K-E1 Section 13.1, "File content is never kept": GitHub answers a directory read of a file
/// with the file, content included, and a read key may ask it.
#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_directory_read_of_a_file_never_keeps_its_content(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok(
        "/contents/deploy/prod.env",
        &file("deploy/prod.env", b"TOKEN=hunter2\n"),
    );
    let read_key = key(&forge, "READ").await;
    let mut variables = json!({ "path": "deploy/prod.env" });
    variables["repository"] = json!(forge.repository);

    for _ in 0..2 {
        let response = forge
            .kit
            .hub
            .post(Some(&read_key), DIRECTORY, variables.clone())
            .await;
        assert_eq!(error(&response).1, "VALIDATION", "{response}");
    }

    assert_eq!(
        forge.heard("/contents/deploy/prod.env").len(),
        2,
        "the second read was served from a held file"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_commits_directory_is_kept_an_hour(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/contents/src", &json!([]));
    let at = |git_ref: &str| json!({ "path": "src", "ref": git_ref });
    forge.ask(DIRECTORY, at(&sha(7))).await;
    forge.ask(DIRECTORY, at("main")).await;
    forge.clock.advance(61);
    forge.ask(DIRECTORY, at(&sha(7))).await;
    forge.ask(DIRECTORY, at("main")).await;
    let heard = |git_ref: &str| {
        let query = format!("?ref={git_ref}");
        let all = forge.heard("/contents/src");
        all.iter().filter(|heard| heard.ends_with(&query)).count()
    };
    assert_eq!((heard(&sha(7)), heard("main")), (1, 2));
    forge.clock.advance(3_600);
    forge.ask(DIRECTORY, at(&sha(7))).await;
    assert_eq!(heard(&sha(7)), 2, "after an hour, read again");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_file_is_closed_to_both_key_scopes_and_the_rest_open(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/contents/a.txt", &file("a.txt", b"a"));
    forge.ok("/contents/", &json!([]));
    forge.ok("/commits", &json!([]));
    forge.ok("/branches", &json!([]));
    forge.ok("/releases", &json!([]));
    let keys = [key(&forge, "READ").await, key(&forge, "WRITE").await];
    let forge = &forge;
    let post = |token: &str, query: &'static str, variables: Value| {
        let mut variables = variables;
        variables["repository"] = json!(forge.repository);
        let token = token.to_owned();
        async move { forge.kit.hub.post(Some(&token), query, variables).await }
    };
    let refused = (
        "This token cannot do that. Sign in to the hub to do it.".to_owned(),
        "FORBIDDEN".to_owned(),
    );
    for secret in &keys {
        let response = post(secret, FILE, json!({ "path": "a.txt" })).await;
        assert_eq!(error(&response), refused);
    }
    let login = post(&forge.kit.token, FILE, json!({ "path": "a.txt" })).await;
    assert_eq!(login["data"]["codeFile"]["text"], "a", "{login}");
    assert_eq!(
        forge.heard("/contents/a.txt").len(),
        1,
        "a key sent nothing"
    );

    let open = [
        ("codeDirectory", DIRECTORY),
        ("commits", COMMITS),
        (
            "branches",
            "query($repository: ID!) { branches(repository: $repository) { edges { cursor } } }",
        ),
        (
            "repositoryReleases",
            "query($repository: ID!) { repositoryReleases(repository: $repository) { edges { cursor } } }",
        ),
    ];
    for secret in keys.iter().chain([&forge.kit.token]) {
        for (root, query) in open {
            let response = post(secret, query, json!({})).await;
            assert!(response["errors"].is_null(), "{root}: {response}");
            assert_ne!(response["data"][root], Value::Null, "{root}");
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_002_the_history_and_code_roots_are_conflict_while_projects_is_off(pool: PgPool) {
    let forge = forge(&pool).await;
    set_projects(&pool, false).await;
    let roots = [
        COMMITS,
        "query($repository: ID!) { branches(repository: $repository) { edges { cursor } } }",
        "query($repository: ID!) { repositoryReleases(repository: $repository) { edges { cursor } } }",
        DIRECTORY,
        FILE,
    ];
    for query in roots {
        let response = forge.ask(query, json!({ "path": "a.txt" })).await;
        assert_eq!(error(&response).1, "CONFLICT", "{query}");
    }
    assert_eq!(forge.live_reads(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_files_content_and_path_reach_no_log_line_and_no_error(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let forge = forge(&pool).await;
    let path = "PATH-MARK-4471/secret.env";
    let content = b"CONTENT-MARK-9902 password=hunter2\n";
    forge.ok("/contents/PATH-MARK-4471/secret.env", &file(path, content));
    forge.ok("/contents/PATH-MARK-4471", &json!([file(path, content)]));
    let shown = forge.ask(FILE, json!({ "path": path })).await;
    assert_eq!(
        shown["data"]["codeFile"]["text"],
        "CONTENT-MARK-9902 password=hunter2\n"
    );
    forge
        .ask(DIRECTORY, json!({ "path": "PATH-MARK-4471" }))
        .await;

    let failing = [
        (FILE, json!({ "path": "PATH-MARK-4471/missing" })),
        (FILE, json!({ "path": "PATH-MARK-4471" })),
        (DIRECTORY, json!({ "path": path })),
        (FILE, json!({ "path": "PATH-MARK-4471/../x" })),
        (FILE, json!({ "path": "a.txt", "ref": "PATH-MARK-4471 x" })),
    ];
    for (query, variables) in failing {
        let response = forge.ask(query, variables).await;
        assert!(response["errors"].is_array(), "{response}");
        let errors = response["errors"].to_string();
        assert!(!errors.contains("MARK"), "{errors}");
    }

    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    assert!(!log.contains("PATH-MARK"), "a path is in the log");
    assert!(!log.contains("CONTENT-MARK"), "content is in the log");
}
