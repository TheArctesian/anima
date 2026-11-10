//! The checkout against a real repository: a local bare repository stands in for GitHub through
//! the repository seam, so these tests run git itself (BLD-040, BLD-041, D-15, D-21).

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    sync::{Arc, Mutex},
};

use crate::builder::{
    child::{Running, Sink},
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
    checkout_into(origin, commit, Arc::new(|_| {})).await
}

async fn checkout_into(origin: &Origin, commit: &str, sink: Sink) -> Result<(), BuilderError> {
    let running = Running::default();
    let workshop = Workshop {
        claim: running.claim(sink).unwrap(),
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

/// The path of the loose object `name` in `clone`.
fn loose(clone: &Path, name: &str) -> PathBuf {
    clone.join(".git/objects").join(&name[..2]).join(&name[2..])
}

/// Makes every object in `clone` loose, as a build can: the packs are unpacked and removed.
fn unpack(clone: &Path) {
    let packs = clone.join(".git/objects/pack");
    let aside = clone.parent().unwrap().join("packs-aside");
    fs::create_dir_all(&aside).unwrap();
    for entry in fs::read_dir(&packs).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "pack") {
            let moved = aside.join(path.file_name().unwrap());
            fs::rename(&path, &moved).unwrap();
            let status = Process::new("git")
                .args(["unpack-objects", "-q"])
                .current_dir(clone)
                .stdin(Stdio::from(fs::File::open(&moved).unwrap()))
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
    for entry in fs::read_dir(&packs).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }
}

/// What a hostile build step can do to the clone without touching its config (D-21): commit an
/// orphan `C`, then overwrite the loose object of `on_main` with one of the same name whose
/// parent is `C`. Returns `C`.
fn forge_parent(origin: &Origin) -> String {
    let clone = &origin.clone;
    git(clone, &["checkout", "-q", "--orphan", "evil"]);
    fs::write(clone.join("evil.txt"), "evil").unwrap();
    git(clone, &["add", "evil.txt"]);
    git(clone, &["commit", "-q", "-m", "evil"]);
    let evil = git(clone, &["rev-parse", "HEAD"]);
    git(clone, &["checkout", "-q", "--detach", &origin.on_main]);
    git(clone, &["branch", "-q", "-D", "evil"]);
    unpack(clone);
    let shown = git(clone, &["cat-file", "-p", &origin.on_main]);
    let tree = shown.lines().next().unwrap().strip_prefix("tree ").unwrap();
    let body = format!(
        "tree {tree}\nparent {evil}\nauthor a <a@a> 1 +0000\ncommitter a <a@a> 1 +0000\n\nmain.txt\n"
    );
    let forged_at = clone.join("forged-commit");
    fs::write(&forged_at, body).unwrap();
    let forged = git(
        clone,
        &[
            "hash-object",
            "-t",
            "commit",
            "-w",
            forged_at.to_str().unwrap(),
        ],
    );
    fs::remove_file(&forged_at).unwrap();
    // The zlib bytes of a loose object do not hold its name: copied under `main`'s name, git
    // reads the forged commit whenever it asks for `main`'s.
    let target = loose(clone, &origin.on_main);
    fs::remove_file(&target).unwrap();
    fs::copy(loose(clone, &forged), &target).unwrap();
    evil
}

/// Adds a commit to `main` of the origin, so `on_main` is no longer its tip; returns it.
fn advance_main(origin: &Origin) -> String {
    let work = origin.clone.parent().unwrap().join("advance");
    let bare = origin.bare.to_str().unwrap();
    git(
        origin.bare.parent().unwrap(),
        &["clone", "-q", bare, work.to_str().unwrap()],
    );
    let tip = commit(&work, "next.txt");
    git(&work, &["push", "-q", bare, "HEAD:refs/heads/main"]);
    tip
}

#[tokio::test]
async fn d_21_a_forged_object_never_lets_a_commit_off_main_be_checked_out() {
    let origin = origin();
    let tip = advance_main(&origin);
    checkout(&origin, &tip).await.unwrap();
    // git checks the hash of the commit it is named, not of the parents it walks to: the forged
    // object is `on_main`, the parent of `main`'s tip.
    let evil = forge_parent(&origin);
    // The forgery works on git itself: without fsck, the ancestry check would pass.
    git(
        &origin.clone,
        &[
            "merge-base",
            "--is-ancestor",
            &evil,
            "refs/remotes/origin/main",
        ],
    );
    std::fs::write(origin.clone.join(".git/planted"), "x").unwrap();
    assert_eq!(checkout(&origin, &evil).await, Err(BuilderError::NotOnMain));
    assert!(!origin.clone.join("evil.txt").exists());
    assert!(
        !origin.clone.join(".git/planted").exists(),
        "the clone was not made anew"
    );
    assert_eq!(checkout(&origin, &tip).await, Ok(()));
    assert_eq!(git(&origin.clone, &["rev-parse", "HEAD"]), tip);
}

#[tokio::test]
async fn d_21_a_clone_that_reads_objects_or_history_from_elsewhere_is_cloned_anew() {
    let origin = origin();
    for file in [
        "commondir",
        "objects/info/alternates",
        "objects/info/commit-graph",
        "objects/info/commit-graphs",
    ] {
        checkout(&origin, &origin.on_main).await.unwrap();
        std::fs::write(origin.clone.join(".git/planted"), "x").unwrap();
        std::fs::write(origin.clone.join(".git").join(file), "").unwrap();
        assert_eq!(checkout(&origin, &origin.on_main).await, Ok(()), "{file}");
        assert!(
            !origin.clone.join(".git/planted").exists(),
            "{file}: the clone was not made anew"
        );
    }
}

#[tokio::test]
async fn d_21_git_steps_print_no_graft_hint() {
    let origin = origin();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&lines);
    let sink: Sink = Arc::new(move |line| seen.lock().unwrap().push(line));
    assert_eq!(checkout_into(&origin, &origin.on_main, sink).await, Ok(()));
    let lines = lines.lock().unwrap();
    assert!(!lines.is_empty(), "git printed nothing to check");
    assert!(
        !lines
            .iter()
            .any(|line| line.to_lowercase().contains("graft")),
        "{lines:?}"
    );
}
