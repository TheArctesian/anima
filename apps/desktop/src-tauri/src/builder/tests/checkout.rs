//! The checkout against a real repository: a local bare repository stands in for GitHub through
//! the repository seam, so these tests run git itself (BLD-040, BLD-041, D-15).

use std::{
    path::{Path, PathBuf},
    process::Command as Process,
    sync::Arc,
};

use crate::builder::{
    child::Running,
    environment::Environment,
    error::BuilderError,
    job::Commit,
    plan::Repository,
    steps::{Workshop, config_is_clones},
};

/// A bare repository whose `main` holds one commit, and a commit on another branch that is
/// not on `main`, as a fork's commit served through the main repository would be.
struct Origin {
    _root: tempfile::TempDir,
    bare: PathBuf,
    clone: PathBuf,
    on_main: String,
    off_main: String,
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Process::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@example.org"])
        .args([
            "-c",
            "protocol.file.allow=always",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit(work: &Path, file: &str) -> String {
    std::fs::write(work.join(file), file).unwrap();
    git(work, &["add", file]);
    git(work, &["commit", "-q", "-m", file]);
    git(work, &["rev-parse", "HEAD"])
}

fn origin() -> Origin {
    let root = tempfile::tempdir().unwrap();
    let bare = root.path().join("anima.git");
    let work = root.path().join("work");
    git(
        root.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    git(root.path(), &["init", "-q", work.to_str().unwrap()]);
    let on_main = commit(&work, "main.txt");
    git(
        &work,
        &["push", "-q", bare.to_str().unwrap(), "HEAD:refs/heads/main"],
    );
    git(&work, &["checkout", "-q", "-b", "fork"]);
    let off_main = commit(&work, "fork.txt");
    git(
        &work,
        &["push", "-q", bare.to_str().unwrap(), "HEAD:refs/heads/fork"],
    );
    let clone = root.path().join("data/builds/anima");
    Origin {
        _root: root,
        bare,
        clone,
        on_main,
        off_main,
    }
}

async fn checkout(origin: &Origin, commit: &str) -> Result<(), BuilderError> {
    let running = Running::default();
    let workshop = Workshop {
        claim: running.claim(Arc::new(|_| {})).unwrap(),
        env: Environment::from_parent(std::env::vars_os()),
        repository: Repository::local(&origin.bare),
        clone: origin.clone.clone(),
    };
    workshop.checkout(&Commit::parse(commit).unwrap()).await
}

#[tokio::test]
async fn bld_041_a_commit_not_on_main_is_refused_and_nothing_is_checked_out() {
    let origin = origin();
    assert_eq!(
        checkout(&origin, &origin.off_main).await,
        Err(BuilderError::NotOnMain)
    );
    assert!(!origin.clone.join("fork.txt").exists());
    assert!(!origin.clone.join("main.txt").exists());
}

#[tokio::test]
async fn bld_040_a_commit_on_main_is_checked_out_with_a_clean_tree() {
    let origin = origin();
    assert_eq!(checkout(&origin, &origin.on_main).await, Ok(()));
    assert!(origin.clone.join("main.txt").is_file());
    assert_eq!(git(&origin.clone, &["rev-parse", "HEAD"]), origin.on_main);
}

#[tokio::test]
async fn d_15_a_clone_whose_config_a_build_changed_is_cloned_anew() {
    let origin = origin();
    checkout(&origin, &origin.on_main).await.unwrap();
    // A build redirects the repository and marks the clone.
    let url = format!("file://{}", origin.bare.display());
    git(
        &origin.clone,
        &["config", "--local", "url.file:///nowhere.insteadOf", &url],
    );
    std::fs::write(origin.clone.join(".git/planted"), "x").unwrap();
    assert_eq!(checkout(&origin, &origin.on_main).await, Ok(()));
    assert!(
        !origin.clone.join(".git/planted").exists(),
        "the clone was not made anew"
    );
    let keys = git(
        &origin.clone,
        &["config", "--local", "--name-only", "--list"],
    );
    assert!(!keys.contains("insteadof"));
}

#[test]
fn d_15_only_the_keys_git_clone_writes_are_trusted() {
    let clone_keys = [
        "core.repositoryformatversion",
        "core.filemode",
        "core.bare",
        "core.logallrefupdates",
        "remote.origin.url",
        "remote.origin.fetch",
        "branch.main.remote",
        "branch.main.merge",
    ]
    .map(str::to_owned);
    assert!(config_is_clones(&clone_keys));
    for foreign in [
        "url.https://evil.example/.insteadof",
        "include.path",
        "core.hookspath",
        "filter.lfs.smudge",
        "core.sshcommand",
    ] {
        let mut keys = clone_keys.to_vec();
        keys.push(foreign.to_owned());
        assert!(!config_is_clones(&keys), "{foreign} was trusted");
    }
}
