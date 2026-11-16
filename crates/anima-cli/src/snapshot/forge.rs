//! Scratch refs on the forge: push one, list them, delete one. Every ref sent is built from
//! [`PREFIX`](super::scratch_ref::PREFIX); nothing is force-pushed and no branch or tag is
//! touched (JOB-016).

use super::SnapshotError;
use super::git::Git;
use super::git_output::{Listed, listed, rejection, safe};
use super::scratch_ref::{PREFIX, ScratchRef, checked_remote};

/// The remote, once it is a name `git remote` lists. A URL or an option is never passed on.
///
/// # Errors
/// [`SnapshotError::BadRemote`] for a name git would misread; [`SnapshotError::UnknownRemote`]
/// when the repository has no remote of that name.
pub fn remote<'a>(git: &Git, name: &'a str) -> Result<&'a str, SnapshotError> {
    let name = checked_remote(name)?;
    let names = git.text(&["remote"])?;
    if names.lines().any(|known| known == name) {
        Ok(name)
    } else {
        Err(SnapshotError::UnknownRemote {
            name: name.to_owned(),
        })
    }
}

/// Pushes `commit` to `scratch` on `remote`, with no `+`, no tags and no submodules.
///
/// # Errors
/// [`SnapshotError::Rejected`] when the remote refuses the ref; [`SnapshotError::Unreachable`]
/// when the push fails for another reason.
pub fn push(
    git: &Git,
    remote: &str,
    commit: &str,
    scratch: &ScratchRef,
) -> Result<(), SnapshotError> {
    let refspec = format!("{commit}:{}", scratch.as_str());
    send(git, remote, scratch, &refspec)
}

/// The scratch refs on `remote`.
///
/// # Errors
/// [`SnapshotError::Unreachable`] when `git ls-remote` fails.
pub fn list(git: &Git, remote: &str) -> Result<Vec<Listed>, SnapshotError> {
    let pattern = format!("{PREFIX}*");
    let ran = git.run(&["ls-remote", "--refs", remote, &pattern])?;
    if !ran.success {
        return Err(SnapshotError::Unreachable {
            remote: remote.to_owned(),
            reason: safe(&ran.stderr),
        });
    }
    Ok(listed(&String::from_utf8_lossy(&ran.stdout)))
}

/// The commit `scratch` holds on `remote`.
///
/// # Errors
/// [`SnapshotError::NoSuchRef`] when the remote has no such ref; as [`list`].
pub fn find(git: &Git, remote: &str, scratch: &ScratchRef) -> Result<Listed, SnapshotError> {
    list(git, remote)?
        .into_iter()
        .find(|row| row.scratch == *scratch)
        .ok_or_else(|| SnapshotError::NoSuchRef {
            remote: remote.to_owned(),
            scratch: scratch.as_str().to_owned(),
        })
}

/// Deletes `scratch` on `remote`.
///
/// # Errors
/// As [`push`].
pub fn delete(git: &Git, remote: &str, scratch: &ScratchRef) -> Result<(), SnapshotError> {
    let refspec = format!(":{}", scratch.as_str());
    send(git, remote, scratch, &refspec)
}

fn send(git: &Git, remote: &str, scratch: &ScratchRef, refspec: &str) -> Result<(), SnapshotError> {
    let ran = git.run(&[
        "push",
        "--porcelain",
        "--no-follow-tags",
        "--recurse-submodules=no",
        remote,
        refspec,
    ])?;
    let stdout = String::from_utf8_lossy(&ran.stdout);
    if let Some(reason) = rejection(&stdout) {
        return Err(SnapshotError::Rejected {
            remote: remote.to_owned(),
            scratch: scratch.as_str().to_owned(),
            reason,
        });
    }
    if ran.success {
        Ok(())
    } else {
        Err(SnapshotError::Unreachable {
            remote: remote.to_owned(),
            reason: safe(&ran.stderr),
        })
    }
}
