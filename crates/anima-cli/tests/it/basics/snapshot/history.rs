//! What a push sends beyond the working tree: commits the remote does not have yet, and files
//! the remote already has (JOB-018).

use super::repos::Repos;

const SECRET: &str = "TOPSECRET-HISTORY\n";

/// `.env` committed, then deleted and the deletion committed; `a.txt` changed after.
fn secret_in_history() -> Repos {
    let repos = Repos::new();
    repos.write(".env", SECRET);
    repos.git(&["add", ".env"]);
    repos.git(&["commit", "--quiet", "-m", "a secret"]);
    repos.git(&["rm", "--quiet", ".env"]);
    repos.git(&["commit", "--quiet", "-m", "the secret removed"]);
    repos.write("a.txt", "changed\n");
    repos
}

#[tokio::test]
async fn job_018_a_secret_in_a_commit_the_remote_lacks_is_refused() {
    let repos = secret_in_history();
    let blob = repos.blob_hash(SECRET);
    let before = repos.state();

    let run = repos.anima(&["snapshot", "push", "--json"]).await;

    assert_eq!(run.code, 5, "{run:?}");
    let error = run.error();
    assert_eq!(error["code"], "FORBIDDEN");
    let message = error["message"].as_str().expect("a message");
    assert!(
        message.contains(".env") && message.contains("origin"),
        "{message}"
    );
    let hint = error["hint"].as_str().expect("a hint");
    assert!(
        hint.contains("a commit that origin does not have"),
        "{hint}"
    );
    run.assert_never_shows("TOPSECRET-HISTORY");
    assert!(
        !repos.has_object(&repos.remote(), &blob),
        "the secret reached the remote"
    );
    assert_eq!(repos.remote_scratch_refs(), "");
    assert_eq!(repos.state(), before);

    let unchanged = secret_in_history();
    unchanged.git(&["checkout", "--quiet", "--", "a.txt"]);
    let run = unchanged.anima(&["snapshot", "push"]).await;
    assert_eq!(
        run.code, 5,
        "pushing HEAD itself sends the history too: {run:?}"
    );
    assert!(!unchanged.has_object(&unchanged.remote(), &blob));
}

#[tokio::test]
async fn job_018_history_the_remote_already_has_is_not_refused() {
    let repos = secret_in_history();
    repos.git(&["push", "--quiet", "origin", "main"]);
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(
        run.code, 0,
        "the deleted .env is already on origin: {run:?}"
    );
    assert_eq!(run.json()["changed"], true);

    // Deleted in the working tree only: the snapshot's tree has no .env, and origin has the
    // commits that do.
    let repos = Repos::new();
    repos.write(".env", SECRET);
    repos.git(&["add", ".env"]);
    repos.git(&["commit", "--quiet", "-m", "a secret"]);
    repos.git(&["push", "--quiet", "origin", "main"]);
    std::fs::remove_file(repos.work().join(".env")).expect("deleted");
    let run = repos.anima(&["snapshot", "push", "--json"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let scratch = run.json()["ref"].as_str().expect("a ref").to_owned();
    let second = repos.second_machine(&scratch);
    let listed = repos.git_in(&second, &["ls-tree", "-r", "--name-only", "FETCH_HEAD"]);
    assert!(!listed.lines().any(|path| path == ".env"), "{listed}");
}

#[tokio::test]
async fn job_018_a_secret_still_in_the_tree_is_refused_when_the_remote_has_it() {
    let repos = Repos::new();
    repos.write("keys/old.pem", SECRET);
    repos.git(&["add", "keys/old.pem"]);
    repos.git(&["commit", "--quiet", "-m", "a key"]);
    repos.git(&["push", "--quiet", "origin", "main"]);
    repos.write("a.txt", "changed\n");

    let run = repos.anima(&["snapshot", "push"]).await;

    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stderr.contains("keys/old.pem"), "{run:?}");
    assert!(run.stderr.contains("git rm --cached"), "{run:?}");
    assert!(run.stderr.contains("No flag sends them"), "{run:?}");
    assert_eq!(repos.remote_scratch_refs(), "");
}
