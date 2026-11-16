//! `anima repo` and `anima code` against the scripted hub (PRJ-036 to PRJ-040, CMD-026,
//! CMD-031).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};
use crate::kit::forge::{
    FULL_NAME, REPOSITORY, SHA, branch, code_directory, code_entry, code_file, commit, connection,
    project, repository_release,
};

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-history-tests")
}

/// Runs `anima <args> --hub <origin>`.
async fn ask(hub: &FakeHub, args: &[&str]) -> crate::kit::Outcome {
    let mut all = args.to_vec();
    all.extend(["--hub", &hub.origin]);
    signed_in().run(&all).await
}

fn variables(hub: &FakeHub, operation: &str) -> Vec<Value> {
    hub.seen()
        .into_iter()
        .filter(|seen| seen.operation == operation)
        .map(|seen| seen.variables)
        .collect()
}

#[tokio::test]
async fn prj_036_repo_commits_names_the_ref_and_prints_a_table_and_json() {
    let hub = FakeHub::start().await;
    project(&hub);
    let commits = [commit(SHA, "Fix the thing")];
    hub.answer(
        "CommitList",
        data(connection("commits", &commits, Some("next-1"))),
    );
    let table = ask(
        &hub,
        &["repo", "commits", FULL_NAME, "--ref", "release/1.x"],
    )
    .await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("0123456"), "{table:?}");
    assert!(table.stdout.contains("Fix the thing"), "{table:?}");
    assert!(table.stdout.contains("octo"), "{table:?}");
    assert!(table.stderr.contains("--after next-1"), "{table:?}");
    assert!(table.stderr.contains("--ref release/1.x"), "{table:?}");
    let sent = &variables(&hub, "CommitList")[0];
    assert_eq!(sent["repository"], REPOSITORY);
    assert_eq!(sent["ref"], "release/1.x");
    assert_eq!(sent["first"], 30);

    let json = ask(&hub, &["repo", "commits", REPOSITORY, "--json", "--fresh"]).await;
    assert_eq!(json.json()["edges"][0]["node"]["sha"], SHA);
    let sent = &variables(&hub, "CommitList")[1];
    assert_eq!((&sent["ref"], &sent["fresh"]), (&Value::Null, &json!(true)));
}

#[tokio::test]
async fn prj_036_repo_commits_checks_asks_for_the_checks_of_each_commit() {
    let hub = FakeHub::start().await;
    let commits = [commit(SHA, "Fix the thing")];
    hub.answer(
        "CommitListChecked",
        data(connection("commits", &commits, None)),
    );
    let run = ask(&hub, &["repo", "commits", REPOSITORY, "--checks"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("FAILED"), "{run:?}");
    assert_eq!(variables(&hub, "CommitListChecked").len(), 1);
    assert_eq!(variables(&hub, "CommitList").len(), 0);
}

#[tokio::test]
async fn cmd_026_repo_commits_all_follows_the_cursor_with_the_largest_page() {
    let hub = FakeHub::start().await;
    let (first, second) = (
        [commit(SHA, "Newer")],
        [commit("b".repeat(40).as_str(), "Older")],
    );
    hub.answer(
        "CommitList",
        data(connection("commits", &first, Some("p2"))),
    );
    hub.answer("CommitList", data(connection("commits", &second, None)));
    let run = ask(&hub, &["repo", "commits", REPOSITORY, "--all", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"].as_array().map(Vec::len), Some(2));
    let sent = variables(&hub, "CommitList");
    assert_eq!(
        (&sent[0]["first"], &sent[1]["after"]),
        (&json!(100), &json!("p2"))
    );
}

#[tokio::test]
async fn prj_037_repo_branches_prints_the_default_and_protected_marks() {
    let hub = FakeHub::start().await;
    let branches = [branch("main", true), branch("feature/x", false)];
    hub.answer(
        "BranchList",
        data(connection("branches", &branches, Some("b2"))),
    );
    let table = ask(&hub, &["repo", "branches", REPOSITORY]).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("feature/x"), "{table:?}");
    assert!(table.stdout.contains("yes"), "{table:?}");
    assert!(table.stderr.contains("--after b2"), "{table:?}");
    assert_eq!(variables(&hub, "BranchList")[0]["first"], 100);

    let json = ask(
        &hub,
        &["repo", "branches", REPOSITORY, "--limit", "5", "--json"],
    )
    .await;
    assert_eq!(json.json()["edges"][0]["node"]["name"], "main");
    assert_eq!(variables(&hub, "BranchList")[1]["first"], 5);
}

#[tokio::test]
async fn prj_038_repo_releases_prints_the_kind_and_pages() {
    let hub = FakeHub::start().await;
    let releases = [repository_release("v1.2.0")];
    hub.answer(
        "RepositoryReleaseList",
        data(connection("repositoryReleases", &releases, Some("r2"))),
    );
    let table = ask(&hub, &["repo", "releases", REPOSITORY]).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("v1.2.0"), "{table:?}");
    assert!(table.stdout.contains("pre-release"), "{table:?}");
    assert!(table.stderr.contains("--after r2"), "{table:?}");
    assert_eq!(variables(&hub, "RepositoryReleaseList")[0]["first"], 20);

    let json = ask(&hub, &["repo", "releases", REPOSITORY, "--json", "--fresh"]).await;
    assert_eq!(json.json()["edges"][0]["node"]["tag"], "v1.2.0");
    assert_eq!(variables(&hub, "RepositoryReleaseList")[1]["fresh"], true);
}

#[tokio::test]
async fn prj_039_code_ls_prints_kind_size_and_name_and_says_when_the_listing_may_be_cut() {
    let hub = FakeHub::start().await;
    let entries = [
        code_entry("lib", "DIRECTORY", 0),
        code_entry("main.rs", "FILE", 2048),
    ];
    hub.answer("CodeDirectoryRead", data(code_directory(&entries, true)));
    let table = ask(&hub, &["code", "ls", REPOSITORY, "src", "--ref", "main"]).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("directory"), "{table:?}");
    assert!(table.stdout.contains("2048"), "{table:?}");
    assert!(table.stdout.contains("main.rs"), "{table:?}");
    assert!(table.stderr.contains("1,000"), "{table:?}");
    let sent = &variables(&hub, "CodeDirectoryRead")[0];
    assert_eq!(
        (&sent["repository"], &sent["ref"], &sent["path"]),
        (&json!(REPOSITORY), &json!("main"), &json!("src"))
    );

    let hub = FakeHub::start().await;
    project(&hub);
    hub.answer("CodeDirectoryRead", data(code_directory(&entries, false)));
    let json = ask(&hub, &["code", "ls", FULL_NAME, "--json"]).await;
    assert_eq!(json.json()["entries"][1]["name"], "main.rs");
    assert_eq!(json.json()["partial"], false);
    assert!(json.stderr.is_empty(), "{json:?}");
    let sent = &variables(&hub, "CodeDirectoryRead")[0];
    assert_eq!((&sent["path"], &sent["ref"]), (&Value::Null, &Value::Null));
}

#[tokio::test]
async fn prj_040_code_show_prints_the_text_byte_for_byte_with_and_without_a_final_newline() {
    for text in ["one\ntwo\n", "no final newline", "tabs\tand\r\nCRLF\n\n"] {
        let hub = FakeHub::start().await;
        hub.answer(
            "CodeFileRead",
            data(code_file("README.md", "TEXT", Some(text))),
        );
        let run = ask(&hub, &["code", "show", REPOSITORY, "README.md"]).await;
        assert_eq!(run.code, 0, "{run:?}");
        assert_eq!(run.stdout, text);
        assert!(run.stderr.is_empty(), "{run:?}");
    }
    let hub = FakeHub::start().await;
    hub.answer(
        "CodeFileRead",
        data(code_file("README.md", "TEXT", Some("x"))),
    );
    let cat = ask(
        &hub,
        &["code", "cat", REPOSITORY, "README.md", "--ref", "v1"],
    )
    .await;
    assert_eq!(cat.stdout, "x");
    let sent = variables(&hub, "CodeFileRead");
    assert_eq!(
        (&sent[0]["ref"], &sent[0]["path"]),
        (&json!("v1"), &json!("README.md"))
    );
}

#[tokio::test]
async fn prj_040_code_show_sends_a_path_with_spaces_and_unicode_as_given() {
    let hub = FakeHub::start().await;
    project(&hub);
    let path = "docs/Mon résumé (final) #1.txt";
    hub.answer("CodeFileRead", data(code_file(path, "TEXT", Some("café"))));
    let run = ask(&hub, &["code", "show", FULL_NAME, path]).await;
    assert_eq!(run.stdout, "café", "{run:?}");
    let sent = &variables(&hub, "CodeFileRead")[0];
    assert_eq!(sent["path"], path);
    assert_eq!(sent["repository"], REPOSITORY, "a name is resolved first");
}

#[tokio::test]
async fn prj_040_code_show_for_binary_or_too_large_prints_the_reason_and_the_link_and_exits_1() {
    for (content, words) in [("BINARY", "is not text"), ("TOO_LARGE", "1,048,576")] {
        let hub = FakeHub::start().await;
        hub.answer("CodeFileRead", data(code_file("logo.png", content, None)));
        let run = ask(&hub, &["code", "show", REPOSITORY, "logo.png"]).await;
        assert_eq!(run.code, 1, "{run:?}");
        assert!(run.stdout.is_empty(), "{run:?}");
        assert!(run.stderr.contains(words), "{run:?}");
        assert!(
            run.stderr
                .contains("https://github.com/acme/anima/blob/main/logo.png"),
            "{run:?}"
        );
    }
}

#[tokio::test]
async fn prj_040_code_show_json_gives_the_whole_object_even_for_a_binary_file() {
    let hub = FakeHub::start().await;
    hub.answer("CodeFileRead", data(code_file("logo.png", "BINARY", None)));
    let run = ask(&hub, &["code", "show", REPOSITORY, "logo.png", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["content"], "BINARY");
    assert_eq!(run.json()["size"], 12);
    let hub = FakeHub::start().await;
    hub.answer("CodeFileRead", data(code_file("a.txt", "TEXT", Some("hi"))));
    let run = ask(&hub, &["code", "show", REPOSITORY, "a.txt", "--json"]).await;
    assert_eq!(run.json()["text"], "hi");
}

#[tokio::test]
async fn prj_040_code_show_passes_on_the_hub_words_for_a_directory_a_link_a_submodule() {
    for words in [
        "That path is a directory, not a file.",
        "That path is a symbolic link, not a file.",
        "That path is a submodule, not a file.",
    ] {
        let hub = FakeHub::start().await;
        hub.answer("CodeFileRead", refusal("VALIDATION", words));
        let run = ask(&hub, &["code", "show", REPOSITORY, "src"]).await;
        assert_eq!(run.code, 2, "{run:?}");
        assert!(run.stdout.is_empty(), "{run:?}");
        assert!(run.stderr.contains(words), "{run:?}");
    }
}

#[tokio::test]
async fn prj_040_code_show_with_an_api_key_shows_the_hub_refusal_and_how_to_sign_in() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CodeFileRead",
        refusal("FORBIDDEN", "Login or browser token only."),
    );
    let key = "anima_historyTestKeyAbcdefghijklmnopqrstuvwxyz0123";
    let run = Anima::new()
        .with("ANIMA_TOKEN", key)
        .run(&["code", "show", REPOSITORY, "a.txt", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stdout.is_empty(), "{run:?}");
    assert!(
        run.stderr.contains("Login or browser token only."),
        "{run:?}"
    );
    assert!(run.stderr.contains("anima auth login"), "{run:?}");
    run.assert_never_shows(key);
}
