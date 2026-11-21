//! The environment of an agent program: the environment of the login of the person, as a login
//! shell makes it, never a secret of Anima (R-NEW-threadsdrive). The daemon runs from a service
//! manager with a short `PATH`, so the runner asks the login shell one time at its start.

use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_threads::drive::Program;

/// The mark before the environment in the output of the shell: a profile may print text first.
const MARK: &str = "ANIMA-LOGIN-ENVIRONMENT-7f3c";

/// The time the login shell has to answer.
const WAIT: Duration = Duration::from_secs(10);

/// Names that never reach an agent program: each name of Anima, and the names by which a
/// program loads code into a different program.
fn left_out(name: &str) -> bool {
    name.starts_with("ANIMA_")
        || name.starts_with("DYLD_")
        || name.starts_with("LD_")
        || matches!(
            name,
            "CLAUDECODE" | "CLAUDE_CODE_ENTRYPOINT" | "CODEX_SANDBOX" | "_"
        )
}

/// The variables the runner sets for each Claude Code program: the state events and the
/// question tool on the pipe (TH10).
pub const CLAUDE_VARIABLES: [(&str, &str); 2] = [
    ("CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS", "1"),
    ("CLAUDE_CODE_ENABLE_ASK_USER_QUESTION_TOOL", "1"),
];

/// The environment from the output of `env -0` after [`MARK`], with the left-out names gone.
#[must_use]
pub fn parse(output: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(output);
    let Some((_, after)) = text.split_once(MARK) else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = after
        .split('\0')
        .filter_map(|pair| pair.split_once('='))
        .filter(|(name, _)| !name.is_empty() && !name.contains('\n') && !left_out(name))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    found.sort();
    found.dedup_by(|a, b| a.0 == b.0);
    found
}

/// The environment of the login of the person, from `$SHELL -l -i -c` with no input and only
/// the names a login needs. When the shell does not answer, the few names the daemon has.
pub async fn environment(daemon: impl Fn(&str) -> Option<String>) -> Vec<(String, String)> {
    let base: Vec<(String, String)> =
        ["HOME", "USER", "LOGNAME", "SHELL", "PATH", "TMPDIR", "LANG"]
            .into_iter()
            .filter_map(|name| Some((name.to_owned(), daemon(name)?)))
            .collect();
    let shell = daemon("SHELL").unwrap_or_else(|| "/bin/sh".to_owned());
    let script = format!("printf '%s' {MARK}; /usr/bin/env -0");
    let run = tokio::process::Command::new(&shell)
        .args(["-l", "-i", "-c", &script])
        .env_clear()
        .envs(base.iter().map(|(name, value)| (name, value)))
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let found = match tokio::time::timeout(WAIT, run).await {
        Ok(Ok(output)) => parse(&output.stdout),
        _ => Vec::new(),
    };
    if found.iter().any(|(name, _)| name == "PATH") {
        found
    } else {
        base.into_iter()
            .filter(|(name, _)| !left_out(name))
            .collect()
    }
}

/// The file name of a program.
const fn file_name(program: Program) -> &'static str {
    match program {
        Program::ClaudeCode => "claude",
        Program::Codex => "codex",
    }
}

/// The absolute path of `program` on the `PATH` of `env`: the first folder that holds an
/// executable file of that name.
#[must_use]
pub fn find(
    program: Program,
    env: &[(String, String)],
    executable: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let path = env.iter().find(|(name, _)| name == "PATH")?;
    path.1
        .split(':')
        .map(Path::new)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(file_name(program)))
        .find(|candidate| executable(candidate))
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
    fn the_environment_is_read_after_the_mark_and_leaves_out_the_names_of_anima() {
        let output = format!(
            "Welcome text\n{MARK}PATH=/usr/bin:/opt/bin\0ANIMA_DAEMON_HOME=/x\0DYLD_INSERT_LIBRARIES=/y\0HOME=/home/a\0EMPTY=\0weird\0"
        );
        let found = parse(output.as_bytes());
        assert_eq!(
            found,
            [
                ("EMPTY".to_owned(), String::new()),
                ("HOME".to_owned(), "/home/a".to_owned()),
                ("PATH".to_owned(), "/usr/bin:/opt/bin".to_owned()),
            ]
        );
        assert!(parse(b"PATH=/bin\0").is_empty(), "no mark, nothing read");
    }

    #[test]
    fn a_program_is_found_in_the_first_absolute_folder_of_the_path() {
        let env = vec![("PATH".to_owned(), "relative:/a:/b".to_owned())];
        let found = find(Program::Codex, &env, |path| {
            path.starts_with("/b") || path.starts_with("relative")
        });
        assert_eq!(found, Some(PathBuf::from("/b/codex")));
        assert_eq!(find(Program::ClaudeCode, &env, |_| false), None);
    }
}
