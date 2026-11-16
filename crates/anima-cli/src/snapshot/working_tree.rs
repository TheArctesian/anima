//! A commit of the working tree (JOB-016) made beside the operator's repository: the tree is
//! built in a temporary index and every new object goes to a temporary object folder that
//! reads the repository's objects as an alternate. The index, the branch, HEAD, the stash, the
//! working tree and the object store stay as they were. A tree with a path that looks like a
//! secret is refused before a commit is made (JOB-017).

use std::path::Path;

use tempfile::TempDir;

use super::SnapshotError;
use super::git::{Git, alternates};
use super::object_paths::tree_entries;
use super::scratch_ref::is_hash;
use super::secret_paths::secret_paths;

/// The identity a snapshot commit gets only when git has none for the operator.
const FALLBACK_NAME: &str = "Anima snapshot";
const FALLBACK_EMAIL: &str = "snapshot@anima.invalid";

const MESSAGE: &str = "Snapshot of the working tree (anima snapshot push)";

/// The commit to push, and the git that can see it.
#[derive(Debug)]
pub struct Snapshot {
    pub commit: String,
    /// False when the working tree matched HEAD, so `commit` is HEAD's own commit.
    pub changed: bool,
    /// Submodules and nested repositories: only their commit is in the snapshot.
    pub gitlinks: Vec<String>,
    /// git with the temporary index and objects; they last as long as this snapshot.
    pub git: Git,
    _scratch: TempDir,
}

/// HEAD's commit and tree; none on a branch with no commit yet.
struct Head {
    commit: String,
    tree: String,
}

/// Commits the working tree: tracked changes and the untracked files the ignore rules let in.
///
/// # Errors
/// [`SnapshotError::SecretPaths`] before any commit is made when the tree holds a path that
/// matches a secret pattern; [`SnapshotError::Git`] when a git step fails.
pub fn take(git: &Git) -> Result<Snapshot, SnapshotError> {
    // Dropped on every return, so the temporary index and objects never stay behind.
    let scratch = tempfile::tempdir().map_err(|error| temporary(&error))?;
    let staged = beside(git, scratch.path())?;
    staged.bytes(&["add", "--all"])?;
    let tree = hash(&staged, &["write-tree"])?;
    let entries = tree_entries(&staged.bytes(&["ls-tree", "-r", "-z", &tree])?);
    let found = secret_paths(entries.iter().map(|entry| entry.path.as_str()));
    if !found.is_empty() {
        return Err(SnapshotError::SecretPaths { paths: found });
    }
    let gitlinks = entries
        .into_iter()
        .filter(|entry| entry.gitlink)
        .map(|entry| entry.path)
        .collect();
    let head = head(&staged)?;
    let (commit, changed) = match head {
        Some(head) if head.tree == tree => (head.commit, false),
        head => (commit(&staged, &tree, head.as_ref())?, true),
    };
    Ok(Snapshot {
        commit,
        changed,
        gitlinks,
        git: staged,
        _scratch: scratch,
    })
}

/// git with an index and an object folder of its own in `scratch`. The index starts as a copy
/// of the operator's, so files git already knows are not read again and sparse-checkout
/// entries keep their flags; with no index yet it starts empty.
fn beside(git: &Git, scratch: &Path) -> Result<Git, SnapshotError> {
    let index = scratch.join("index");
    let objects = scratch.join("objects");
    std::fs::create_dir(&objects).map_err(|error| temporary(&error))?;
    let own_index = absolute_git_path(git, "index")?;
    if Path::new(&own_index).is_file() {
        std::fs::copy(&own_index, &index).map_err(|error| temporary(&error))?;
    }
    let own_objects = absolute_git_path(git, "objects")?;
    let before = std::env::var("GIT_ALTERNATE_OBJECT_DIRECTORIES").ok();
    Ok(git
        .with("GIT_INDEX_FILE", index.as_os_str())
        .with("GIT_OBJECT_DIRECTORY", objects.as_os_str())
        .with(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            alternates(&own_objects, before.as_deref()),
        ))
}

fn absolute_git_path(git: &Git, name: &str) -> Result<String, SnapshotError> {
    let path = git.text(&["rev-parse", "--path-format=absolute", "--git-path", name])?;
    Ok(path.trim_end().to_owned())
}

fn head(git: &Git) -> Result<Option<Head>, SnapshotError> {
    let ran = git.run(&["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])?;
    if !ran.success {
        return Ok(None);
    }
    let commit = String::from_utf8_lossy(&ran.stdout).trim_end().to_owned();
    let tree = hash(git, &["rev-parse", "--verify", "HEAD^{tree}"])?;
    Ok(Some(Head { commit, tree }))
}

/// A commit of `tree` on top of HEAD, by the operator's git identity or, when git has none,
/// the fallback.
fn commit(git: &Git, tree: &str, head: Option<&Head>) -> Result<String, SnapshotError> {
    let mut args = vec!["commit-tree", tree, "-m", MESSAGE];
    if let Some(head) = head {
        args.extend(["-p", head.commit.as_str()]);
    }
    let known = ["GIT_AUTHOR_IDENT", "GIT_COMMITTER_IDENT"]
        .into_iter()
        .map(|ident| git.run(&["var", ident]).map(|ran| ran.success))
        .collect::<Result<Vec<bool>, _>>()?;
    let git = if known.iter().all(|known| *known) {
        git.clone()
    } else {
        git.with("GIT_AUTHOR_NAME", FALLBACK_NAME)
            .with("GIT_AUTHOR_EMAIL", FALLBACK_EMAIL)
            .with("GIT_COMMITTER_NAME", FALLBACK_NAME)
            .with("GIT_COMMITTER_EMAIL", FALLBACK_EMAIL)
    };
    hash(&git, &args)
}

/// The one hash `git <args>` prints.
fn hash(git: &Git, args: &[&str]) -> Result<String, SnapshotError> {
    let text = git.text(args)?.trim_end().to_owned();
    if is_hash(&text) {
        Ok(text)
    } else {
        Err(SnapshotError::Git {
            step: args.first().copied().unwrap_or("git").to_owned(),
            reason: "it did not print a hash".to_owned(),
        })
    }
}

fn temporary(error: &std::io::Error) -> SnapshotError {
    SnapshotError::Temporary {
        reason: error.to_string(),
    }
}
