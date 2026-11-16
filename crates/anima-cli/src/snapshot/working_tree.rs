//! A commit of the working tree (JOB-016) made beside the operator's repository state: the
//! tree is built in a temporary index, so the index, the branch, HEAD, the stash and the
//! working tree stay as they were. The tree is refused before anything leaves when a path
//! looks like a secret (JOB-017).

use std::collections::BTreeSet;
use std::path::Path;

use super::SnapshotError;
use super::git::Git;
use super::git_output::paths;
use super::secret_paths::secret_paths;

/// The identity a snapshot commit gets only when git has none for the operator.
const FALLBACK_NAME: &str = "Anima snapshot";
const FALLBACK_EMAIL: &str = "snapshot@anima.invalid";

const MESSAGE: &str = "Snapshot of the working tree (anima snapshot push)";

/// The commit to push.
#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub commit: String,
    /// False when the working tree matched HEAD, so `commit` is HEAD's own commit.
    pub changed: bool,
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
    // Dropped on every return, so the temporary index never stays behind.
    let folder = tempfile::tempdir().map_err(|error| SnapshotError::Temporary {
        reason: error.to_string(),
    })?;
    let index = folder.path().join("index");
    seed(git, &index)?;
    let staged = git.with("GIT_INDEX_FILE", index.as_os_str());
    // Before `add`, so the content of a secret file is not even copied into the object store.
    refuse(would_add(&staged)?)?;
    staged.bytes(&["add", "--all"])?;
    let tree = staged.text(&["write-tree"])?.trim_end().to_owned();
    // The tree itself is what leaves, so it is judged too.
    refuse(paths(&git.bytes(&[
        "ls-tree",
        "-r",
        "-z",
        "--name-only",
        &tree,
    ])?))?;
    let head = head(git)?;
    if let Some(head) = head.as_ref().filter(|head| head.tree == tree) {
        return Ok(Snapshot {
            commit: head.commit.clone(),
            changed: false,
        });
    }
    let commit = commit(git, &tree, head.as_ref())?;
    Ok(Snapshot {
        commit,
        changed: true,
    })
}

/// Starts the temporary index as a copy of the operator's, so files that git already knows
/// are not read again and sparse-checkout entries keep their flags. No index yet: it starts
/// empty.
fn seed(git: &Git, index: &Path) -> Result<(), SnapshotError> {
    let own = git.text(&["rev-parse", "--git-path", "index"])?;
    let own = git.top().join(own.trim_end());
    if own.is_file() {
        std::fs::copy(&own, index).map_err(|error| SnapshotError::Temporary {
            reason: error.to_string(),
        })?;
    }
    Ok(())
}

/// The paths `add --all` would leave in the index: those git knows or the ignore rules let
/// in, without the ones deleted from the working tree.
fn would_add(staged: &Git) -> Result<Vec<String>, SnapshotError> {
    let known = staged.bytes(&[
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ])?;
    let deleted: BTreeSet<String> = paths(&staged.bytes(&["ls-files", "-z", "--deleted"])?)
        .into_iter()
        .collect();
    let known: BTreeSet<String> = paths(&known).into_iter().collect();
    Ok(known.difference(&deleted).cloned().collect())
}

/// Refuses when a path matches a secret pattern. Only names are read, never the files.
fn refuse(all: Vec<String>) -> Result<(), SnapshotError> {
    let found = secret_paths(all.iter().map(String::as_str));
    if found.is_empty() {
        Ok(())
    } else {
        Err(SnapshotError::SecretPaths { paths: found })
    }
}

fn head(git: &Git) -> Result<Option<Head>, SnapshotError> {
    let ran = git.run(&["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])?;
    if !ran.success {
        return Ok(None);
    }
    let commit = String::from_utf8_lossy(&ran.stdout).trim_end().to_owned();
    let tree = git
        .text(&["rev-parse", "--verify", "HEAD^{tree}"])?
        .trim_end()
        .to_owned();
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
    Ok(git.text(&args)?.trim_end().to_owned())
}
