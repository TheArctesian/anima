//! `anima snapshot push` in repositories that are not one plain checkout: a sparse checkout, a
//! linked worktree, tags that git would follow, submodules and nested repositories (JOB-016).

use super::repos::Repos;

fn files_at(repos: &Repos, scratch: &str) -> (std::path::PathBuf, String) {
    let second = repos.second_machine(scratch);
    let listed = repos.git_in(&second, &["ls-tree", "-r", "--name-only", "FETCH_HEAD"]);
    (second, listed)
}

#[tokio::test]
async fn job_016_a_sparse_checkout_keeps_the_files_outside_the_cone() {
    let repos = Repos::new();
    repos.write("inside/x.txt", "x\n");
    repos.write("outside/y.txt", "y\n");
    repos.git(&["add", "--all"]);
    repos.git(&["commit", "--quiet", "-m", "two folders"]);
    repos.git(&["sparse-checkout", "set", "inside"]);
    assert!(
        !repos.work().join("outside/y.txt").exists(),
        "not checked out"
    );
    repos.write("inside/x.txt", "x, changed\n");
    let before = repos.state();

    let run = repos.anima(&["snapshot", "push", "--json"]).await;

    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(repos.state(), before);
    let scratch = run.json()["ref"].as_str().expect("a ref").to_owned();
    let (second, listed) = files_at(&repos, &scratch);
    assert!(
        listed.lines().any(|path| path == "outside/y.txt"),
        "{listed}"
    );
    let show = |path: &str| repos.git_in(&second, &["show", &format!("FETCH_HEAD:{path}")]);
    assert_eq!(show("outside/y.txt"), "y\n");
    assert_eq!(show("inside/x.txt"), "x, changed\n");
}

#[tokio::test]
async fn job_016_a_linked_worktree_is_snapshotted_without_touching_the_repository() {
    let repos = Repos::new();
    repos.git(&["worktree", "add", "--quiet", "../linked", "-b", "linked"]);
    let linked = repos.work().join("../linked");
    std::fs::write(linked.join("only-here.txt"), "linked\n").expect("a file");
    let before = repos.state();

    let run = repos
        .anima_in(&linked, &["snapshot", "push", "--json"])
        .await;

    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(repos.state(), before, "the shared object store changed");
    let scratch = run.json()["ref"].as_str().expect("a ref").to_owned();
    let (_, listed) = files_at(&repos, &scratch);
    assert!(
        listed.lines().any(|path| path == "only-here.txt"),
        "{listed}"
    );
}

#[tokio::test]
async fn job_016_no_tag_follows_the_snapshot_to_the_remote() {
    let repos = Repos::new();
    repos.git(&["config", "push.followTags", "true"]);
    repos.git(&["tag", "--annotate", "v1", "--message", "v1"]);
    repos.write("a.txt", "changed\n");

    let run = repos.anima(&["snapshot", "push"]).await;

    assert_eq!(run.code, 0, "{run:?}");
    let tags = repos.git_in(&repos.remote(), &["for-each-ref", "refs/tags/"]);
    assert_eq!(tags, "", "a tag reached the remote");
}

#[tokio::test]
async fn job_016_a_submodule_is_not_pushed_and_its_path_is_named() {
    let repos = Repos::new();
    let library = repos.work().join("../library.git");
    let library_path = library.to_str().expect("UTF-8").to_owned();
    let seed = repos.work().join("../library");
    let commit = [
        "-c",
        "user.name=L",
        "-c",
        "user.email=l@example.invalid",
        "commit",
    ];
    let commit_empty = |folder: &std::path::Path, message: &str| {
        let args = [&commit[..], &["--quiet", "--allow-empty", "-m", message]].concat();
        repos.git_in(folder, &args);
    };
    repos.git(&["init", "--quiet", "--bare", "-b", "main", &library_path]);
    repos.git(&["init", "--quiet", "-b", "main", "../library"]);
    commit_empty(&seed, "one");
    repos.git_in(&seed, &["push", "--quiet", &library_path, "main"]);
    repos.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        &library_path,
        "lib",
    ]);
    repos.git(&["commit", "--quiet", "-m", "the library"]);
    repos.git(&["push", "--quiet", "origin", "main"]);
    let lib = repos.work().join("lib");
    commit_empty(&lib, "two");
    let unpushed = repos.git_in(&lib, &["rev-parse", "HEAD"]).trim().to_owned();
    repos.git(&["config", "push.recurseSubmodules", "on-demand"]);

    let run = repos.anima(&["snapshot", "push"]).await;

    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        !repos.has_object(&library, &unpushed),
        "the submodule was pushed"
    );
    assert!(
        run.stderr.contains("submodule or nested repository"),
        "{run:?}"
    );
    assert!(run.stderr.contains("lib"), "{run:?}");
}

#[tokio::test]
async fn job_016_a_nested_repository_is_named_as_a_gitlink() {
    let repos = Repos::new();
    let nested = repos.work().join("nested");
    std::fs::create_dir(&nested).expect("a folder");
    repos.git_in(&nested, &["init", "--quiet", "-b", "main"]);
    std::fs::write(nested.join("inner.txt"), "inner\n").expect("a file");
    repos.git_in(&nested, &["add", "inner.txt"]);
    repos.git_in(
        &nested,
        &[
            "-c",
            "user.name=N",
            "-c",
            "user.email=n@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "n",
        ],
    );

    let run = repos.anima(&["snapshot", "push", "--json"]).await;

    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["gitlinks"], serde_json::json!(["nested"]));
    let text = repos.anima(&["snapshot", "push"]).await;
    assert!(
        text.stderr
            .contains("their uncommitted changes are not sent"),
        "{text:?}"
    );
    assert!(text.stderr.contains("nested"), "{text:?}");
}
