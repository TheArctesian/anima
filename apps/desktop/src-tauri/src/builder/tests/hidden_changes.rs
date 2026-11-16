//! Changes a build hides from `git status` (S12 security review M-1, M-2): an index entry marked
//! `--skip-worktree` or `--assume-unchanged`, an exclude rule, a changed recipe. Each is undone by
//! the checkout or refused before a recipe runs. A local bare repository stands in for GitHub.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::checkout::git;
use crate::builder::{
    carried::{link, tree},
    child::Running,
    environment::Environment,
    error::BuilderError,
    job::{Commit, Target, Version},
    plan::Repository,
    steps::Workshop,
};

/// Fake recipes that write their name to `order` beside the clone; `link/build.sh` also writes
/// the header of an x86-64 ELF program where the bundle step takes it.
const FILES: [(&str, &str); 6] = [
    (
        "apps/desktop/src-tauri/database-tree/build.sh",
        "echo tree >> ../order\n",
    ),
    (
        "apps/desktop/src-tauri/database-tree/postgresql.pin",
        "version=17.12\n",
    ),
    (
        "link/build.sh",
        "echo link >> ../order\nmkdir -p link/bin\nprintf '\\177ELF\\002\\001\\001\\000\\000\\000\\000\\000\\000\\000\\000\\000\\002\\000\\076\\000' > link/bin/anima-link-linux-amd64\n",
    ),
    ("link/go.mod", "module example.org/link\n\ngo 1.21\n"),
    ("link/go.sum", ""),
    ("main.txt", "committed\n"),
];

/// A bare repository whose `main` holds `FILES`, and the clone's path, in a folder reached
/// through no link (the tree's work folder is made beside the clone).
struct Origin {
    _root: tempfile::TempDir,
    bare: PathBuf,
    clone: PathBuf,
    commit: Commit,
}

fn origin() -> Origin {
    let root = tempfile::tempdir().unwrap();
    let top = fs::canonicalize(root.path()).unwrap();
    let bare = top.join("anima.git");
    let work = top.join("work");
    git(&top, &["init", "-q", "--bare", bare.to_str().unwrap()]);
    git(&top, &["init", "-q", work.to_str().unwrap()]);
    for (path, text) in FILES {
        fs::create_dir_all(work.join(path).parent().unwrap()).unwrap();
        fs::write(work.join(path), text).unwrap();
    }
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "files"]);
    git(
        &work,
        &["push", "-q", bare.to_str().unwrap(), "HEAD:refs/heads/main"],
    );
    let commit = Commit::parse(&git(&work, &["rev-parse", "HEAD"])).unwrap();
    let clone = top.join("data/builds/anima");
    Origin {
        _root: root,
        bare,
        clone,
        commit,
    }
}

fn workshop<'a>(origin: &Origin, running: &'a Running) -> Workshop<'a> {
    Workshop {
        claim: running.claim(Arc::new(|_| {})).unwrap(),
        env: Environment::from_parent(std::env::vars_os()),
        repository: Repository::local(&origin.bare),
        clone: origin.clone.clone(),
    }
}

async fn checked_out() -> Origin {
    let origin = origin();
    let running = Running::default();
    let workshop = workshop(&origin, &running);
    assert_eq!(workshop.checkout(&origin.commit).await, Ok(()));
    origin
}

#[tokio::test]
async fn m_1_an_edit_hidden_by_an_index_mark_is_refused_and_the_checkout_undoes_it() {
    for mark in ["--skip-worktree", "--assume-unchanged"] {
        let origin = checked_out().await;
        let running = Running::default();
        let workshop = workshop(&origin, &running);
        git(&origin.clone, &["update-index", mark, "main.txt"]);
        fs::write(origin.clone.join("main.txt"), "changed by a build\n").unwrap();
        assert_eq!(git(&origin.clone, &["status", "--porcelain"]), "", "{mark}");
        assert_eq!(
            workshop.at(&origin.commit).await,
            Err(BuilderError::Dirty),
            "{mark}"
        );
        assert_eq!(workshop.checkout(&origin.commit).await, Ok(()), "{mark}");
        assert_eq!(
            fs::read_to_string(origin.clone.join("main.txt")).unwrap(),
            "committed\n",
            "{mark}"
        );
        assert_eq!(workshop.at(&origin.commit).await, Ok(()), "{mark}");
    }
}

#[tokio::test]
async fn m_1_an_excluded_untracked_file_is_refused_and_does_not_survive_the_checkout() {
    let origin = checked_out().await;
    let running = Running::default();
    let workshop = workshop(&origin, &running);
    assert!(
        !origin.clone.join(".git/info/exclude").exists(),
        "the checkout removed it"
    );
    fs::create_dir_all(origin.clone.join(".git/info")).unwrap();
    fs::write(origin.clone.join(".git/info/exclude"), "*\n").unwrap();
    fs::write(origin.clone.join("link/go.work"), "go 1.21\n").unwrap();
    assert_eq!(git(&origin.clone, &["status", "--porcelain"]), "");
    assert_eq!(workshop.at(&origin.commit).await, Err(BuilderError::Dirty));
    assert_eq!(workshop.checkout(&origin.commit).await, Ok(()));
    assert!(!origin.clone.join("link/go.work").exists());
    assert_eq!(workshop.at(&origin.commit).await, Ok(()));
}

#[tokio::test]
async fn m_1_a_recipe_that_is_not_the_committed_one_fails_its_check() {
    let origin = checked_out().await;
    let running = Running::default();
    let workshop = workshop(&origin, &running);
    let commit = &origin.commit;
    assert_eq!(workshop.committed(commit, &link::COMMITTED).await, Ok(()));
    assert_eq!(workshop.committed(commit, &tree::COMMITTED).await, Ok(()));
    let recipe = origin.clone.join("link/build.sh");
    git(
        &origin.clone,
        &["update-index", "--skip-worktree", "link/build.sh"],
    );
    fs::write(&recipe, "echo changed\n").unwrap();
    assert_eq!(
        workshop.committed(commit, &link::COMMITTED).await,
        Err(BuilderError::Dirty)
    );
    // The same bytes through a link are not the committed file.
    let copy = origin.clone.parent().unwrap().join("build.sh");
    fs::write(&copy, FILES[2].1).unwrap();
    fs::remove_file(&recipe).unwrap();
    std::os::unix::fs::symlink(&copy, &recipe).unwrap();
    assert_eq!(
        workshop.committed(commit, &link::COMMITTED).await,
        Err(BuilderError::Dirty)
    );
    fs::remove_file(&recipe).unwrap();
    assert_eq!(
        workshop.committed(commit, &link::COMMITTED).await,
        Err(BuilderError::Dirty)
    );
}

#[tokio::test]
async fn m_2_the_tree_recipe_then_the_link_run_before_cargo_and_a_go_workspace_stops_the_link() {
    let origin = checked_out().await;
    let running = Running::default();
    let workshop = workshop(&origin, &running);
    let version = Version::parse("0.1.12").unwrap();
    let order = origin.clone.parent().unwrap().join("order");
    // Linux: no signing identity. The repository has no Cargo.toml, so `cargo build` fails.
    let result = workshop
        .bundle(&origin.commit, Target::LinuxX86_64, &version)
        .await;
    assert_eq!(result, Err(BuilderError::StepFailed));
    assert_eq!(fs::read_to_string(&order).unwrap(), "tree\nlink\n");
    fs::remove_file(&order).unwrap();
    fs::write(origin.clone.join("link/go.work"), "go 1.21\n").unwrap();
    assert_eq!(
        workshop.link(&origin.commit, Target::LinuxX86_64).await,
        Err(BuilderError::Dirty)
    );
    assert!(!order.exists(), "the link recipe did not run");
    fs::remove_file(origin.clone.join("link/go.work")).unwrap();
    fs::create_dir_all(origin.clone.join("link/vendor")).unwrap();
    assert_eq!(
        workshop.link(&origin.commit, Target::LinuxX86_64).await,
        Err(BuilderError::Dirty)
    );
    assert!(Path::new(&origin.clone).join("link/go.mod").is_file());
}
