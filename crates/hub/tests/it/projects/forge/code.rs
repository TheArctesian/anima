//! PRJ-033, K-E1 Section 13.8: one directory of a repository and one file in K1's shape, with
//! every kind of answer GitHub gives: text, binary, too large, a directory or a file where the
//! other was asked, a link, a submodule, the 1,000-entry mark and GitHub's failures.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{BASE, forge, ok, reply};
use crate::hub::{error, start};

pub const DIRECTORY: &str = "query($repository: ID!, $ref: String, $path: String) { \
    codeDirectory(repository: $repository, ref: $ref, path: $path) { repository ref path \
    entries { name path kind size sha } partial read { stale } } }";

pub const FILE: &str = "query($repository: ID!, $ref: String, $path: String!) { \
    codeFile(repository: $repository, ref: $ref, path: $path) { repository ref path name sha \
    size content text url readAt } }";

/// A file as GitHub's contents answer gives it, at `path`, holding `bytes`.
pub fn file(path: &str, bytes: &[u8]) -> Value {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    json!({
        "type": "file", "encoding": "base64", "size": bytes.len(),
        "name": path.rsplit('/').next().unwrap(), "path": path, "content": STANDARD.encode(bytes),
        "sha": "a".repeat(40), "download_url": "https://raw.example.invalid/x"
    })
}

/// A directory listing of `count` files.
fn listing(count: usize) -> Value {
    Value::Array(
        (0..count)
            .map(|n| {
                json!({ "type": "file", "size": 1, "name": format!("f{n:04}"),
                    "path": format!("f{n:04}"), "sha": "b".repeat(40),
                    "download_url": "https://raw.example.invalid/x" })
            })
            .collect(),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_directory_comes_back_directories_first_with_each_kind(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/contents/",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_contents_directory.json"
        )),
    );
    let response = forge.ask(DIRECTORY, json!({})).await;
    let directory = &response["data"]["codeDirectory"];
    assert_eq!(
        directory["repository"],
        Value::from(forge.repository.clone())
    );
    assert_eq!(directory["ref"], "main", "{response}");
    assert_eq!(directory["path"], "");
    assert_eq!(directory["partial"], false);
    let entry = |name: &str, kind: &str, size: u64, digit: &str| json!({ "name": name, "path": name, "kind": kind, "size": size, "sha": digit.repeat(20) });
    assert_eq!(
        directory["entries"],
        json!([
            entry("assets", "DIRECTORY", 0, "b6"),
            entry("src", "DIRECTORY", 0, "b2"),
            entry("Alpha.md", "FILE", 340, "b3"),
            entry("alpha.md", "FILE", 1, "b7"),
            entry("latest", "SYMLINK", 0, "b5"),
            entry("vendor-lib", "SUBMODULE", 0, "b4"),
            entry("zeta.txt", "FILE", 12, "b1"),
        ])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_directory_of_1000_entries_may_have_more(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/contents/full", &listing(1_000));
    forge.ok("/contents/short", &listing(999));
    let full = forge.ask(DIRECTORY, json!({ "path": "full" })).await;
    assert_eq!(full["data"]["codeDirectory"]["partial"], true);
    let entries = full["data"]["codeDirectory"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1_000);
    let short = forge.ask(DIRECTORY, json!({ "path": "short" })).await;
    assert_eq!(short["data"]["codeDirectory"]["partial"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_text_file_comes_back_with_its_page(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/contents/docs/README.md",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_contents_file.json"
        )),
    );
    let response = forge.ask(FILE, json!({ "path": "docs/README.md" })).await;
    assert_eq!(
        response["data"]["codeFile"],
        json!({
            "repository": forge.repository, "ref": "main", "path": "docs/README.md",
            "name": "README.md", "sha": "a1".repeat(20), "size": 94, "content": "TEXT",
            "text": "# Sample\n\nHello, world. Caf\u{e9} \u{2603} notes.\nSecond line of the \
                sample file, long enough to wrap.\n",
            "url": "https://github.com/octo/widgets/blob/main/docs/README.md",
            "readAt": start().to_rfc3339()
        })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_binary_and_too_large_files_come_back_without_content(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok(
        "/contents/logo.gif",
        &file("logo.gif", b"GIF89a\x00\x01binary"),
    );
    forge.ok("/contents/bad.txt", &file("bad.txt", b"caf\xe9"));
    let mut big = file("big.bin", b"");
    big["size"] = json!(2_000_000);
    forge.ok("/contents/big.bin", &big);
    let mut edge = file("edge.txt", &vec![b'x'; 1_000_000]);
    edge["size"] = json!(1_000_000);
    forge.ok("/contents/edge.txt", &edge);
    forge
        .kit
        .stub
        .endless(&format!("{BASE}/contents/endless.log"));

    let shape = |response: &Value| {
        let found = &response["data"]["codeFile"];
        (
            found["content"].clone(),
            found["text"].clone(),
            found["size"].clone(),
        )
    };
    let ask = |path: &'static str| forge.ask(FILE, json!({ "path": path }));
    assert_eq!(
        shape(&ask("logo.gif").await),
        (json!("BINARY"), Value::Null, json!(14))
    );
    assert_eq!(
        shape(&ask("bad.txt").await),
        (json!("BINARY"), Value::Null, json!(4))
    );
    let big = ask("big.bin").await;
    assert_eq!(
        shape(&big),
        (json!("TOO_LARGE"), Value::Null, json!(2_000_000))
    );
    let edge = ask("edge.txt").await;
    assert_eq!(
        edge["data"]["codeFile"]["content"], "TEXT",
        "1,000,000 bytes is read"
    );
    let endless = ask("endless.log").await;
    assert_eq!(shape(&endless), (json!("TOO_LARGE"), Value::Null, json!(0)));
    assert_eq!(endless["data"]["codeFile"]["name"], "endless.log");
    assert_eq!(endless["data"]["codeFile"]["path"], "endless.log");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_directory_where_a_file_was_asked_and_the_reverse_are_validation(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/contents/src", &listing(2));
    forge.ok("/contents/a.txt", &file("a.txt", b"a"));
    let cases = [
        (FILE, "src", "That path is a directory, not a file."),
        (DIRECTORY, "a.txt", "That path is a file, not a directory."),
    ];
    for (query, path, words) in cases {
        let response = forge.ask(query, json!({ "path": path })).await;
        assert_eq!(error(&response).1, "VALIDATION", "{response}");
        let fields = &response["errors"][0]["extensions"]["fields"];
        assert_eq!(fields, &json!([{ "field": "path", "message": words }]));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_link_and_a_submodule_are_not_files_and_a_link_is_never_followed(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/contents/latest",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_contents_symlink.json"
        )),
    );
    forge.reply(
        "/contents/vendor-lib",
        ok(include_str!(
            "../../../../../providers/github/tests/fixtures/live_contents_submodule.json"
        )),
    );
    for (path, words) in [
        (
            "latest",
            "That path is a link the hub does not follow, not a file.",
        ),
        ("vendor-lib", "That path is a submodule, not a file."),
    ] {
        let response = forge.ask(FILE, json!({ "path": path })).await;
        let fields = &response["errors"][0]["extensions"]["fields"];
        assert_eq!(
            fields,
            &json!([{ "field": "path", "message": words }]),
            "{response}"
        );
    }
    assert!(
        forge.heard("/contents/zeta.txt").is_empty(),
        "the target is not read"
    );
}

/// GitHub answers a link to a file in the same repository with that file (decisions item 13).
#[sqlx::test(migrations = "./migrations")]
async fn prj_033_a_link_github_resolved_shows_the_path_github_read(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok(
        "/contents/docs/latest",
        &file("docs/v2/notes.txt", b"notes"),
    );
    let response = forge.ask(FILE, json!({ "path": "docs/latest" })).await;
    let found = &response["data"]["codeFile"];
    assert_eq!(found["path"], "docs/v2/notes.txt", "{response}");
    assert_eq!(found["name"], "notes.txt");
    assert_eq!(found["text"], "notes");
    assert_eq!(
        found["url"],
        "https://github.com/octo/widgets/blob/main/docs/v2/notes.txt"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_code_failures_have_their_code_and_fixed_words(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.reply(
        "/contents/gone.txt",
        reply(404, &[], "{\"message\":\"Not Found\"}"),
    );
    forge.reply("/contents/gone", reply(404, &[], "{}"));
    forge.kit.stub.endless(&format!("{BASE}/contents/huge"));
    let not_found = (
        "GitHub has no such path, or the App may not read it.".to_owned(),
        "NOT_FOUND".to_owned(),
    );
    let file = forge.ask(FILE, json!({ "path": "gone.txt" })).await;
    assert_eq!(error(&file), not_found);
    let directory = forge.ask(DIRECTORY, json!({ "path": "gone" })).await;
    assert_eq!(error(&directory), not_found);
    let huge = forge.ask(DIRECTORY, json!({ "path": "huge" })).await;
    assert_eq!(
        error(&huge),
        (
            "GitHub's answer is larger than the hub reads.".to_owned(),
            "UNAVAILABLE".to_owned()
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_rate_limit_on_a_file_says_until_when(pool: PgPool) {
    let forge = forge(&pool).await;
    let reset = (start() + chrono::TimeDelta::minutes(15))
        .timestamp()
        .to_string();
    let limited = [
        ("x-ratelimit-remaining", "0"),
        ("x-ratelimit-reset", reset.as_str()),
    ];
    forge.reply("/contents/a.txt", reply(403, &limited, "{}"));
    let response = forge.ask(FILE, json!({ "path": "a.txt" })).await;
    assert_eq!(
        error(&response),
        (
            "GitHub limits requests until 12:15.".to_owned(),
            "UNAVAILABLE".to_owned()
        )
    );
}
