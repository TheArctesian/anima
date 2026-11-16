//! Whether a file the carried steps run or read is, byte for byte, the one the job's commit
//! holds. A build can hide a changed file from `git status` (`--skip-worktree`,
//! `--assume-unchanged`, an exclude rule), so each such file is hashed and compared with the
//! commit's blob directly before its use. Pure.

use std::path::Path;

use crate::builder::{
    job::Commit,
    plan::{Command, git},
};

/// The blob name of the bytes at `path` on disk, with no clean filter or line-end change.
pub fn on_disk(clone: &Path, path: &str) -> Command {
    git(clone, &["hash-object", "--no-filters", "--", path])
}

/// The blob name of `path` in `commit`.
pub fn in_commit(clone: &Path, commit: &Commit, path: &str) -> Command {
    git(
        clone,
        &[
            "rev-parse",
            "--verify",
            &format!("{}:{path}", commit.as_str()),
        ],
    )
}

/// Whether both commands printed one and the same object name: 40 or 64 lower-case hex digits.
pub fn same_blob(on_disk: &[String], in_commit: &[String]) -> bool {
    object_name(on_disk).is_some_and(|name| object_name(in_commit) == Some(name))
}

/// The one object name `lines` holds, if that is all they hold.
fn object_name(lines: &[String]) -> Option<&str> {
    match lines {
        [line]
            if matches!(line.len(), 40 | 64)
                && line
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Some(line)
        }
        _ => None,
    }
}
