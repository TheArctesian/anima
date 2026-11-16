//! `anima snapshot push`: what is sent, what stays as it was, and what is refused (JOB-017,
//! JOB-018).

use super::repos::Repos;

/// The working tree has a tracked change, a new file, an ignored file and a staged change
/// that was changed again after `git add`.
fn with_changes() -> Repos {
    let repos = Repos::new();
    repos.write("a.txt", "one, changed\n");
    repos.write("src/new.txt", "new\n");
    repos.write("debug.log", "ignored\n");
    repos.write("b.txt", "staged\n");
    repos.git(&["add", "b.txt"]);
    repos.write("b.txt", "staged, then changed again\n");
    repos
}

fn left_nothing_behind(repos: &Repos) {
    let left: Vec<_> = std::fs::read_dir(repos.temporary())
        .expect("the temporary folder")
        .collect();
    assert!(left.is_empty(), "temporary files stayed: {left:?}");
}

#[tokio::test]
async fn job_017_push_sends_the_working_tree_and_leaves_the_repository_as_it_was() {
    let repos = with_changes();
    let before = repos.state();
    let head = repos.git(&["rev-parse", "HEAD"]).trim().to_owned();

    let run = repos.anima(&["snapshot", "push", "--json"]).await;

    assert_eq!(run.code, 0, "{run:?}");
    let pushed = run.json();
    let commit = pushed["commit"].as_str().expect("a commit");
    let scratch = format!("refs/anima/wip/{}", &commit[..16]);
    assert_eq!(pushed["ref"], scratch.as_str());
    assert_eq!(pushed["remote"], "origin");
    assert_eq!(pushed["changed"], true);
    assert_eq!(
        pushed["fetch"],
        format!("git fetch origin {scratch}").as_str()
    );
    assert_eq!(repos.state(), before, "the operator's repository changed");
    left_nothing_behind(&repos);
    assert_eq!(repos.remote_scratch_refs(), format!("{commit} {scratch}\n"));
    let branches = repos.git_in(&repos.remote(), &["for-each-ref", "refs/heads/"]);
    assert!(
        branches.contains(&head) && branches.lines().count() == 1,
        "{branches}"
    );

    let second = repos.second_machine(&scratch);
    let show = |path: &str| repos.git_in(&second, &["show", &format!("FETCH_HEAD:{path}")]);
    assert_eq!(show("a.txt"), "one, changed\n");
    assert_eq!(show("src/new.txt"), "new\n");
    assert_eq!(show("b.txt"), "staged, then changed again\n");
    assert_eq!(show(".gitignore"), "*.log\n");
    let ignored = repos
        .git_command(&second, &["cat-file", "-e", "FETCH_HEAD:debug.log"])
        .output()
        .expect("git starts");
    assert!(!ignored.status.success(), "the ignored file was sent");
    assert_eq!(
        repos.git_in(&second, &["rev-parse", "FETCH_HEAD^"]).trim(),
        head,
        "the snapshot sits on HEAD"
    );
}

#[tokio::test]
async fn job_017_push_prints_the_ref_and_the_fetch_line_for_a_person() {
    let repos = with_changes();
    let run = repos.anima(&["snapshot", "push"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stderr
            .contains("Pushed a snapshot of the working tree."),
        "{run:?}"
    );
    for label in ["Ref", "Commit", "Remote", "Fetch with"] {
        assert!(run.stdout.contains(label), "{run:?}");
    }
    assert!(
        run.stdout.contains("git fetch origin refs/anima/wip/"),
        "{run:?}"
    );
}

#[tokio::test]
async fn job_017_with_no_changes_head_itself_is_pushed() {
    let repos = Repos::new();
    let head = repos.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let pushed = run.json();
    assert_eq!(pushed["commit"], head.as_str());
    assert_eq!(pushed["changed"], false);
    assert_eq!(
        pushed["ref"],
        format!("refs/anima/wip/{}", &head[..16]).as_str()
    );

    let text = repos.anima(&["snapshot", "push"]).await;
    assert_eq!(text.code, 0, "pushing the same snapshot again: {text:?}");
    assert!(text.stderr.contains("no changes"), "{text:?}");
}

#[tokio::test]
async fn job_018_each_secret_pattern_is_refused_and_nothing_is_pushed() {
    for path in [
        ".env",
        "apps/web/.env.local",
        "certs/server.pem",
        "tls.key",
        "client.p12",
        "deploy/prod.env",
        "home/.ssh/id_ed25519",
        "id_rsa",
        ".envrc",
        "deploy/secrets/hub.toml",
        "gcp/credentials.json",
        "home/.netrc",
    ] {
        let repos = with_changes();
        repos.write(path, "TOPSECRET-VALUE\n");
        let before = repos.state();

        let run = repos.anima(&["snapshot", "push", "--json"]).await;

        assert_eq!(run.code, 5, "{path}: {run:?}");
        let error = run.error();
        assert_eq!(error["code"], "FORBIDDEN", "{path}");
        assert!(
            error["message"].as_str().unwrap().contains(path),
            "{path}: {error}"
        );
        run.assert_never_shows("TOPSECRET-VALUE");
        assert_eq!(
            repos.remote_scratch_refs(),
            "",
            "{path}: something was pushed"
        );
        assert_eq!(repos.state(), before, "{path}: the repository changed");
        left_nothing_behind(&repos);
        let blob = repos.blob_hash("TOPSECRET-VALUE\n");
        assert!(
            !repos.has_object(&repos.work(), &blob),
            "{path}: the secret entered the local object store"
        );
    }
}

#[tokio::test]
async fn job_018_a_refusal_names_twenty_paths_and_counts_the_rest() {
    let repos = Repos::new();
    for n in 0..25 {
        repos.write(&format!("k{n:02}.key"), "TOPSECRET-VALUE\n");
    }
    let run = repos.anima(&["snapshot", "push"]).await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(
        run.stderr.contains("k19.key") && !run.stderr.contains("k20.key"),
        "{run:?}"
    );
    assert!(run.stderr.contains("and 5 more"), "{run:?}");
}

#[tokio::test]
async fn job_018_templates_and_ignored_secrets_pass() {
    let repos = Repos::new();
    repos.write(".env.example", "KEY=\n");
    repos.write("apps/.env.sample", "KEY=\n");
    repos.write(".gitignore", "*.log\n.env\n");
    repos.write(".env", "TOPSECRET-VALUE\n");
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let scratch = run.json()["ref"].as_str().expect("a ref").to_owned();
    let second = repos.second_machine(&scratch);
    let listed = repos.git_in(&second, &["ls-tree", "-r", "--name-only", "FETCH_HEAD"]);
    assert!(listed.contains(".env.example") && listed.contains("apps/.env.sample"));
    assert!(!listed.lines().any(|line| line == ".env"), "{listed}");
}

#[tokio::test]
async fn job_017_with_no_git_identity_the_fallback_signs_the_commit() {
    let repos = Repos::new();
    repos.git(&["config", "--unset", "user.name"]);
    repos.git(&["config", "--unset", "user.email"]);
    repos.git(&["config", "user.useConfigOnly", "true"]);
    repos.write("a.txt", "changed\n");
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let commit = run.json()["commit"].as_str().expect("a commit").to_owned();
    let author = repos.git_in(
        &repos.remote(),
        &["log", "-1", "--format=%an <%ae>", &commit],
    );
    assert_eq!(author.trim(), "Anima snapshot <snapshot@anima.invalid>");

    let repos = Repos::new();
    repos.write("a.txt", "changed\n");
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    let commit = run.json()["commit"].as_str().expect("a commit").to_owned();
    let author = repos.git_in(
        &repos.remote(),
        &["log", "-1", "--format=%an <%ae>", &commit],
    );
    assert_eq!(author.trim(), "Operator <operator@example.invalid>");
}
