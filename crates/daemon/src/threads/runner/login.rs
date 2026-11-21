//! The environment of an agent program: the environment of the login of the person, as a login
//! shell makes it, never a secret of Anima (R-308). The daemon runs from a service
//! manager with a short `PATH`, so the runner asks the login shell one time at its start.

use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_threads::drive::Program;

/// The mark before the environment in the output of the shell: a profile may print text first.
const MARK: &str = "ANIMA-LOGIN-ENVIRONMENT-7f3c";
/// The mark after it: the runner reads up to it and does not wait for the end of the output,
/// which a process that the profile started in the background can hold open.
const END: &str = "ANIMA-LOGIN-ENVIRONMENT-END-7f3c";

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

/// The environment from the output of `env -0` between [`MARK`] and [`END`], with the left-out
/// names gone, and each value that is a value of a name of Anima in `secrets` gone (THR-040).
/// For a name that comes twice, the last value counts, as in a shell.
#[must_use]
pub fn parse(output: &[u8], secrets: &[String]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(output);
    let Some((_, after)) = text.split_once(MARK) else {
        return Vec::new();
    };
    let Some((body, _)) = after.split_once(END) else {
        return Vec::new();
    };
    let mut found: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (name, value) in body.split('\0').filter_map(|pair| pair.split_once('=')) {
        let secret = !value.is_empty() && secrets.iter().any(|secret| secret == value);
        if !name.is_empty() && !name.contains('\n') && !left_out(name) && !secret {
            found.insert(name.to_owned(), value.to_owned());
        }
    }
    found.into_iter().collect()
}

/// The environment of the login of the person, from `$SHELL -l -i -c` with no input and only
/// the names a login needs; `daemon` is the environment of the daemon. When the shell does not
/// answer, the few names the daemon has, and a warning in the log.
pub async fn environment(daemon: &[(String, String)]) -> Vec<(String, String)> {
    let base = base_of(daemon);
    let found = asked(daemon, &base).await;
    if found.iter().any(|(name, _)| name == "PATH") {
        return found;
    }
    tracing::warn!("the environment of the login has no PATH; the daemon's few names are used");
    let secrets = secrets_of(daemon);
    base.into_iter()
        .filter(|(name, value)| !left_out(name) && !secrets.contains(value))
        .collect()
}

/// The environment that the login shell gives, or nothing, with the reason in the log.
async fn asked(daemon: &[(String, String)], base: &[(String, String)]) -> Vec<(String, String)> {
    match ask_shell(&shell_of(daemon), base).await {
        Ok(output) => parse(&output, &secrets_of(daemon)),
        Err(why) => {
            tracing::warn!(%why, "the login shell did not give its environment");
            Vec::new()
        }
    }
}

fn value_of(daemon: &[(String, String)], wanted: &str) -> Option<String> {
    daemon
        .iter()
        .find(|(name, _)| name == wanted)
        .map(|(_, value)| value.clone())
}

/// The names of the daemon that a login shell needs to start.
fn base_of(daemon: &[(String, String)]) -> Vec<(String, String)> {
    ["HOME", "USER", "LOGNAME", "SHELL", "PATH", "TMPDIR", "LANG"]
        .into_iter()
        .filter_map(|name| Some((name.to_owned(), value_of(daemon, name)?)))
        .collect()
}

/// The values of the names of Anima in the environment of the daemon.
fn secrets_of(daemon: &[(String, String)]) -> Vec<String> {
    daemon
        .iter()
        .filter(|(name, value)| name.starts_with("ANIMA_") && !value.is_empty())
        .map(|(_, value)| value.clone())
        .collect()
}

/// The shell of the login: `SHELL`, else the shell of the account, else `/bin/sh`.
fn shell_of(daemon: &[(String, String)]) -> String {
    value_of(daemon, "SHELL")
        .or_else(|| value_of(daemon, "USER").and_then(|user| login_shell(&user)))
        .unwrap_or_else(|| "/bin/sh".to_owned())
}

/// The output of the login shell up to [`END`]. The shell runs in a process group of its own
/// with no input, so that a profile cannot read the terminal of a person; the group ends after
/// the answer or at [`WAIT`], with each process that the profile left in it.
async fn ask_shell(shell: &str, base: &[(String, String)]) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt as _;
    let script = format!("printf '%s' {MARK}; /usr/bin/env -0; printf '%s' {END}");
    let mut child = tokio::process::Command::new(shell)
        .args(["-l", "-i", "-c", &script])
        .env_clear()
        .envs(base.iter().map(|(name, value)| (name, value)))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()?;
    let group = child.id();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no output"))?;
    let read = async {
        let mut output = Vec::new();
        let mut part = [0_u8; 8192];
        loop {
            let count = stdout.read(&mut part).await?;
            output.extend_from_slice(&part[..count]);
            if count == 0
                || output
                    .windows(END.len())
                    .any(|window| window == END.as_bytes())
            {
                return Ok::<_, std::io::Error>(output);
            }
        }
    };
    let answer = tokio::time::timeout(WAIT, read).await;
    if let Some(group) = group
        .and_then(|pid| i32::try_from(pid).ok())
        .and_then(rustix::process::Pid::from_raw)
    {
        // A group that already ended is what this wants: the error is not a fault.
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
    }
    let _ = child.wait().await;
    answer.map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "no answer in time"))?
}

/// The login shell of `user` from the account database, for a daemon that has no `SHELL`.
fn login_shell(user: &str) -> Option<String> {
    if cfg!(target_os = "macos") {
        let output = std::process::Command::new("/usr/bin/dscl")
            .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        return text
            .split_whitespace()
            .nth(1)
            .filter(|shell| shell.starts_with('/'))
            .map(str::to_owned);
    }
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd
        .lines()
        .map(|line| line.split(':').collect::<Vec<_>>())
        .find(|fields| fields.first() == Some(&user))
        .and_then(|fields| fields.get(6).map(|shell| (*shell).to_owned()))
        .filter(|shell| shell.starts_with('/'))
}

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

/// The absolute path of `program` on the `PATH` of `env`: the first folder that holds an
/// executable file of that name. Then the [`USUAL`] folders, with `~` as the `HOME` of `env`,
/// each only when `trusted` holds for it (a folder of the person or of the system, that no other
/// user can write).
#[must_use]
pub fn find(
    program: Program,
    env: &[(String, String)],
    executable: impl Fn(&Path) -> bool,
    trusted: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let home = env
        .iter()
        .find(|(name, _)| name == "HOME")
        .map(|(_, home)| home.as_str());
    let usual = USUAL
        .iter()
        .filter_map(|folder| match folder.strip_prefix("~/") {
            Some(below) => home.map(|home| Path::new(home).join(below)),
            None => Some(PathBuf::from(folder)),
        })
        .filter(|folder| trusted(folder));
    env.iter()
        .find(|(name, _)| name == "PATH")
        .into_iter()
        .flat_map(|(_, path)| path.split(':').map(PathBuf::from))
        .chain(usual)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(file_name(program)))
        .find(|candidate| executable(candidate))
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
    fn the_environment_is_read_after_the_mark_and_leaves_out_the_names_of_anima() {
        let output = format!(
            "Welcome text\n{MARK}PATH=/usr/bin:/opt/bin\0ANIMA_DAEMON_HOME=/x\0DYLD_INSERT_LIBRARIES=/y\0HOME=/home/a\0EMPTY=\0weird\0{END}bye from .zlogout"
        );
        let found = parse(output.as_bytes(), &[]);
        assert_eq!(
            found,
            [
                ("EMPTY".to_owned(), String::new()),
                ("HOME".to_owned(), "/home/a".to_owned()),
                ("PATH".to_owned(), "/usr/bin:/opt/bin".to_owned()),
            ]
        );
        assert!(
            parse(b"PATH=/bin\0", &[]).is_empty(),
            "no mark, nothing read"
        );
    }

    #[test]
    fn thr_040_a_value_of_a_secret_of_anima_and_a_cut_output_are_left_out() {
        let output = format!("{MARK}A=1\0COPY=s3cret\0A=2\0{END}");
        let found = parse(output.as_bytes(), &["s3cret".to_owned()]);
        assert_eq!(
            found,
            [("A".to_owned(), "2".to_owned())],
            "the last value counts"
        );
        let cut = format!("{MARK}PATH=/bin\0");
        assert!(
            parse(cut.as_bytes(), &[]).is_empty(),
            "no end mark, nothing read"
        );
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
