//! The copies of Claude Code and Codex on this computer, and the one a start runs (TH97).

use std::path::{Path, PathBuf};

use agent_threads::drive::Program;

/// The file name of a program.
const fn file_name(program: Program) -> &'static str {
    match program {
        Program::ClaudeCode => "claude",
        Program::Codex => "codex",
    }
}

/// The folders where Claude Code and Codex are often installed, below the home folder or not.
/// They are looked in after the `PATH` of the login, for a login shell that gave no `PATH`.
const USUAL: [&str; 6] = [
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "~/.local/bin",
    "~/.claude/local",
    "~/.bun/bin",
    "~/.npm-global/bin",
];

/// Where an application keeps its own copy of a program, after the [`USUAL`] folders.
const fn in_applications(program: Program) -> &'static [&'static str] {
    match program {
        Program::ClaudeCode => &[],
        Program::Codex => {
            &["/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS"]
        }
    }
}

/// The absolute path of `program` on the `PATH` of `env`: the first of [`copies`].
#[must_use]
pub fn find(
    program: Program,
    env: &[(String, String)],
    executable: impl Fn(&Path) -> bool,
    trusted: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    copies(program, env, executable, trusted).into_iter().next()
}

/// Each copy of `program` on this computer, in the order a start chooses: each folder of the
/// `PATH` of `env` that holds an executable file of that name, then the [`USUAL`] folders, with
/// `~` as the `HOME` of `env`, then the folders of applications; a folder that is not on the
/// `PATH` only when `trusted` holds for it (a folder of the person or of the system, that no
/// other user can write). A file that two folders reach is one copy.
#[must_use]
pub fn copies(
    program: Program,
    env: &[(String, String)],
    executable: impl Fn(&Path) -> bool,
    trusted: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let home = env
        .iter()
        .find(|(name, _)| name == "HOME")
        .map(|(_, home)| home.as_str());
    let usual = USUAL
        .iter()
        .chain(in_applications(program))
        .filter_map(|folder| match folder.strip_prefix("~/") {
            Some(below) => home.map(|home| Path::new(home).join(below)),
            None => Some(PathBuf::from(folder)),
        })
        .filter(|folder| trusted(folder));
    let mut seen = std::collections::BTreeSet::new();
    let found: Vec<PathBuf> = env
        .iter()
        .find(|(name, _)| name == "PATH")
        .into_iter()
        .flat_map(|(_, path)| path.split(':').map(PathBuf::from))
        .chain(usual)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(file_name(program)))
        .filter(|candidate| executable(candidate))
        .filter(|candidate| {
            seen.insert(std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone()))
        })
        .collect();
    // The first copy is the one a terminal runs (THR-091). Each other copy can be asked for its
    // version or named by a start, so its folder must be one no other user can write.
    let mut each = found.into_iter();
    each.next()
        .into_iter()
        .chain(each.filter(|copy| copy.parent().is_some_and(&trusted)))
        .collect()
}

/// The copy of the program that a start runs: the one it named, when that is one of the
/// `copies` the runner found, else the first of them (TH97). A path that is not a found copy is
/// refused, so that a start never runs a file of its choice.
///
/// # Errors
///
/// `ProgramNotFound` when no copy was found, or the named path is not one of them.
pub fn chosen_copy(
    named: Option<&Path>,
    copies: &[PathBuf],
) -> Result<PathBuf, agent_threads::control::Refusal> {
    let Some(named) = named else {
        return copies
            .first()
            .cloned()
            .ok_or(agent_threads::control::Refusal::ProgramNotFound);
    };
    let real = |path: &Path| std::fs::canonicalize(path).ok();
    let wanted = real(named).ok_or(agent_threads::control::Refusal::ProgramNotFound)?;
    copies
        .iter()
        .find(|copy| real(copy).as_ref() == Some(&wanted))
        .cloned()
        .ok_or(agent_threads::control::Refusal::ProgramNotFound)
}

/// Whether `folder` belongs to this user or to the system and no other user can write it.
#[must_use]
pub fn trusted(folder: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    let me = rustix::process::getuid().as_raw();
    std::fs::metadata(folder).is_ok_and(|meta| {
        meta.is_dir() && (meta.uid() == me || meta.uid() == 0) && meta.mode() & 0o022 == 0
    })
}

/// `env` with the folder of `program` last on its `PATH` when the `PATH` does not hold it, so
/// that a program found in a [`USUAL`] folder also finds the programs next to it, and the order
/// of the login stays first.
#[must_use]
pub fn with_folder_of(mut env: Vec<(String, String)>, program: &Path) -> Vec<(String, String)> {
    let Some(folder) = program
        .parent()
        .and_then(Path::to_str)
        .filter(|folder| !folder.contains(':'))
    else {
        return env;
    };
    match env.iter_mut().find(|(name, _)| name == "PATH") {
        Some((_, path)) if path.split(':').any(|known| known == folder) => {}
        Some((_, path)) => *path = format!("{path}:{folder}"),
        None => env.push(("PATH".to_owned(), folder.to_owned())),
    }
    env
}

/// Whether `path` is a file that this user may run.
#[must_use]
pub fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_not_on_the_path_is_found_in_a_usual_folder_and_its_folder_goes_on_the_path() {
        let env = vec![
            ("HOME".to_owned(), "/home/a".to_owned()),
            ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        ];
        let at_home = |path: &Path| path == Path::new("/home/a/.local/bin/claude");
        let found = find(Program::ClaudeCode, &env, at_home, |_| true);
        assert_eq!(found, Some(PathBuf::from("/home/a/.local/bin/claude")));
        assert_eq!(
            find(Program::ClaudeCode, &env, at_home, |_| false),
            None,
            "a usual folder that another user can write is not used"
        );
        let on_path = find(
            Program::Codex,
            &env,
            |path| path == Path::new("/bin/codex"),
            |_| false,
        );
        assert_eq!(
            on_path,
            Some(PathBuf::from("/bin/codex")),
            "the PATH comes first"
        );
        let env = with_folder_of(env, Path::new("/home/a/.local/bin/claude"));
        assert!(env.contains(&(
            "PATH".to_owned(),
            "/usr/bin:/bin:/home/a/.local/bin".to_owned()
        )));
        let again = with_folder_of(env.clone(), Path::new("/usr/bin/claude"));
        assert_eq!(again, env, "a folder on the PATH is not added twice");
    }

    #[test]
    fn a_program_is_found_in_the_first_absolute_folder_of_the_path() {
        let env = vec![("PATH".to_owned(), "relative:/a:/b".to_owned())];
        let found = find(
            Program::Codex,
            &env,
            |path| path.starts_with("/b") || path.starts_with("relative"),
            |_| false,
        );
        assert_eq!(found, Some(PathBuf::from("/b/codex")));
        assert_eq!(find(Program::ClaudeCode, &env, |_| false, |_| false), None);
    }
}
