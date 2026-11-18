//! Finds the repository of a session's working folder (AIS-002, R-134): the nearest folder at
//! or above it, below the home folder, that has a `.git`; for a worktree, the main repository
//! through the `gitdir` link and its `commondir`. It reads only those two small files and the
//! git `config` (AIS-022), never through a link. The name is read by
//! `daemon_core::agent_usage::repository`.

use std::fs;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use daemon_core::agent_usage::repository::{gitdir_link, origin_url, remote_name};

/// The most of a `.git` link or a `commondir` file read.
const LINK_LIMIT: u64 = 4 * 1024;

/// The most of a git `config` read.
const CONFIG_LIMIT: u64 = 64 * 1024;

/// The folders on macOS whose files the system guards: reading one would ask the operator for
/// access, so no git file is read there.
const GUARDED: [&str; 4] = [
    "Documents",
    "Desktop",
    "Downloads",
    "Library/Mobile Documents",
];

/// What the git files of a working folder say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Repository {
    /// `host/owner/name`; `None` with no remote `origin` that names a host.
    pub name: Option<String>,
    /// The folder of the main repository, for the exclusions.
    pub main: Option<PathBuf>,
}

/// Where the git files may be read.
pub(super) struct Bounds<'a> {
    pub home: &'a Path,
    pub macos: bool,
}

/// The repository of `cwd`. A folder that is gone, has no `.git` up to the home folder, or is
/// guarded on macOS gives none.
pub(super) fn repository(cwd: &Path, bounds: &Bounds) -> Repository {
    for folder in cwd.ancestors() {
        if folder == bounds.home {
            break;
        }
        if guarded(folder, bounds) {
            return Repository::default();
        }
        let dot_git = folder.join(".git");
        let Ok(kind) = fs::symlink_metadata(&dot_git) else {
            continue;
        };
        let common = if kind.is_dir() {
            dot_git
        } else if kind.is_file() {
            let Some(common) = common_folder(folder, &dot_git) else {
                return Repository::default();
            };
            common
        } else {
            return Repository::default();
        };
        let name = read_small(&common.join("config"), CONFIG_LIMIT)
            .as_deref()
            .and_then(origin_url)
            .and_then(remote_name);
        let main = common
            .file_name()
            .is_some_and(|last| last == ".git")
            .then(|| common.parent().map(Path::to_path_buf))
            .flatten();
        return Repository { name, main };
    }
    Repository::default()
}

/// The common git folder of a worktree whose `.git` file is `dot_git`, in `folder`.
fn common_folder(folder: &Path, dot_git: &Path) -> Option<PathBuf> {
    let link = read_small(dot_git, LINK_LIMIT)?;
    let gitdir = folder.join(gitdir_link(&link)?);
    match read_small(&gitdir.join("commondir"), LINK_LIMIT) {
        Some(common) => Some(lexical(&gitdir.join(common.trim()))),
        None => Some(lexical(&gitdir)),
    }
}

/// `path` with each `..` taking out the folder before it, as git reads its relative links.
fn lexical(path: &Path) -> PathBuf {
    let mut plain = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                plain.pop();
            }
            Component::CurDir => {}
            other => plain.push(other),
        }
    }
    plain
}

fn guarded(folder: &Path, bounds: &Bounds) -> bool {
    bounds.macos
        && GUARDED
            .iter()
            .any(|guarded| folder.starts_with(bounds.home.join(guarded)))
}

/// The text of a regular file (not a link), at most `limit` bytes; `None` when it is not one
/// or cannot be read.
fn read_small(path: &Path, limit: u64) -> Option<String> {
    if !fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    fs::File::open(path)
        .ok()?
        .take(limit)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
        fs::write(path, text).expect("written");
    }

    const CONFIG: &str =
        "[remote \"origin\"]\n\turl = https://me:ghp_secret@github.com/me/app.git\n";

    #[test]
    fn ais_002_a_folder_and_its_worktrees_name_the_main_repository() {
        let home = tempfile::tempdir().expect("a home");
        let bounds = Bounds {
            home: home.path(),
            macos: false,
        };
        let main = home.path().join("code/app");
        write(&main.join(".git/config"), CONFIG);
        let expected = Repository {
            name: Some("github.com/me/app".to_owned()),
            main: Some(main.clone()),
        };
        assert_eq!(
            repository(&main.join("crates/x"), &bounds),
            expected,
            "a subfolder"
        );
        let worktree = home.path().join("elsewhere/wt");
        write(
            &worktree.join(".git"),
            "gitdir: ../../code/app/.git/worktrees/wt\n",
        );
        write(&main.join(".git/worktrees/wt/commondir"), "../..\n");
        assert_eq!(repository(&worktree, &bounds), expected, "a git worktree");
        let gone = main.join(".claude/worktrees/feature");
        assert_eq!(
            repository(&gone, &bounds),
            expected,
            "a worktree that is gone"
        );
    }

    #[test]
    fn ais_002_no_repository_above_home_on_a_link_or_in_a_guarded_folder() {
        let home = tempfile::tempdir().expect("a home");
        write(&home.path().join(".git/config"), CONFIG);
        let bounds = Bounds {
            home: home.path(),
            macos: true,
        };
        assert_eq!(
            repository(&home.path().join("notes"), &bounds),
            Repository::default()
        );
        let guarded = home.path().join("Documents/app");
        write(&guarded.join(".git/config"), CONFIG);
        assert_eq!(repository(&guarded, &bounds), Repository::default());
        let linked = home.path().join("linked");
        fs::create_dir_all(&linked).expect("a folder");
        std::os::unix::fs::symlink(guarded.join(".git"), linked.join(".git")).expect("a link");
        assert_eq!(repository(&linked, &bounds), Repository::default());
    }
}
