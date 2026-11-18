//! The reader on a synthetic home folder (never the operator's): which files it reads, how a
//! session gathers its sub-agents, the exclusions, and the reads after the first.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use daemon_core::agent_usage::{SessionDay, Source};

use super::{AgentSessions, user_home};
use crate::config::agent_usage::UsageSettings;

const MARKER: &str = "MARKER-7f3a9c-SECRET";

fn claude_line(cwd: &Path, output: u64) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-11-08T09:00:00Z","cwd":"{}","message":{{"model":"claude-opus-5-5","content":[{{"type":"text","text":"{MARKER}"}}],"usage":{{"output_tokens":{output}}}}}}}"#,
        cwd.display()
    )
}

fn codex_meta(id: &str, parent: Option<&str>, cwd: &Path) -> String {
    let parent = parent.map_or(String::new(), |p| format!(r#","parent_thread_id":"{p}""#));
    format!(
        r#"{{"timestamp":"2026-11-08T10:00:00Z","type":"session_meta","payload":{{"id":"{id}","cwd":"{}"{parent}}}}}"#,
        cwd.display()
    )
}

const CODEX_CALL: &str = r#"{"timestamp":"2026-11-08T10:00:01Z","type":"response_item","payload":{"type":"function_call"}}"#;

fn write(path: &Path, lines: &[String]) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
    let mut text = lines.join("\n");
    text.push('\n');
    fs::write(path, text).expect("written");
}

fn append(path: &Path, line: &str) {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open");
    writeln!(file, "{line}").expect("appended");
}

fn scan(sessions: &mut AgentSessions) -> Vec<SessionDay> {
    sessions.scan(&AtomicBool::new(false))
}

fn find<'a>(days: &'a [SessionDay], source: Source, id: &str) -> &'a SessionDay {
    days.iter()
        .find(|day| day.source == source && day.id == id)
        .expect("the session")
}

/// A home with a Claude Code session and its sub-agent, a Codex thread and its sub-agent, a
/// session in an excluded folder, a settings file of Claude Code, and a link to a session.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("a home");
    let root = home.path();
    let app = root.join("code/app");
    let config = "[remote \"origin\"]\n\turl = https://me:ghp_x@github.com/me/app.git\n";
    write(&app.join(".git/config"), &[config.to_owned()]);
    let project = root.join(".claude/projects/-code-app");
    write(&project.join("s1.jsonl"), &[claude_line(&app, 10)]);
    write(
        &project.join("s1/subagents/agent-a1.jsonl"),
        &[claude_line(&app, 5)],
    );
    write(
        &project.join("s1/subagents/journal.jsonl"),
        &[claude_line(&app, 99)],
    );
    let private = root.join("private/app");
    write(
        &root.join(".claude/projects/-private/s2.jsonl"),
        &[claude_line(&private, 7)],
    );
    write(
        &root.join(".claude/settings.json"),
        &[claude_line(&app, 99)],
    );
    let codex = root.join(".codex/sessions/2026/11/08");
    let codex_parent = [codex_meta("t1", None, &app), CODEX_CALL.to_owned()];
    write(
        &codex.join("rollout-2026-11-08T10-00-00-t1.jsonl"),
        &codex_parent,
    );
    let codex_child = [codex_meta("t2", Some("t1"), &app), CODEX_CALL.to_owned()];
    write(
        &codex.join("rollout-2026-11-08T10-05-00-t2.jsonl"),
        &codex_child,
    );
    write(&root.join("elsewhere/s3.jsonl"), &[claude_line(&app, 1000)]);
    std::os::unix::fs::symlink(root.join("elsewhere/s3.jsonl"), project.join("s3.jsonl"))
        .expect("a link");
    home
}

fn settings(home: &Path) -> UsageSettings {
    UsageSettings {
        excluded_folders: vec![home.join("private")],
        ..UsageSettings::default()
    }
}

#[test]
fn ais_018_a_scan_counts_each_session_with_its_sub_agents_and_its_repository() {
    let home = home();
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings(home.path()));
    let days = scan(&mut sessions);
    assert_eq!(
        days.len(),
        2,
        "s1 and t1; s2 excluded; the link and other files not read"
    );
    let claude = find(&days, Source::ClaudeCode, "s1");
    assert_eq!(
        claude.models[0].tokens.output, 15,
        "the sub-agent counts in its session"
    );
    assert_eq!(claude.repository.as_deref(), Some("github.com/me/app"));
    let codex = find(&days, Source::Codex, "t1");
    assert_eq!(
        codex.tool_calls, 2,
        "the sub-agent thread counts under its parent"
    );
    assert!(!format!("{days:?}").contains(MARKER));
    assert!(!format!("{days:?}").contains(&home.path().display().to_string()));
}

#[test]
fn ais_001_a_scan_reads_only_what_changed() {
    let home = home();
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings(home.path()));
    scan(&mut sessions);
    assert_eq!(scan(&mut sessions).len(), 0, "nothing changed");
    let app = home.path().join("code/app");
    let agent = home
        .path()
        .join(".claude/projects/-code-app/s1/subagents/agent-a1.jsonl");
    append(&agent, &claude_line(&app, 1));
    let days = scan(&mut sessions);
    assert_eq!(days.len(), 1, "only the session whose file grew");
    assert_eq!(
        days[0].models[0].tokens.output, 16,
        "the new line once, the old kept"
    );
}

#[test]
fn ais_023_an_excluded_repository_is_not_counted() {
    let home = home();
    let settings = UsageSettings {
        excluded_repositories: vec!["github.com/me/app".to_owned()],
        ..UsageSettings::default()
    };
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings.clone());
    assert_eq!(
        scan(&mut sessions).len(),
        0,
        "s2's repository cannot be read, so it may be the excluded one (AU-5 security review)"
    );
    let other = "[remote \"origin\"]\n\turl = git@github.com:me/other.git\n";
    write(
        &home.path().join("private/app/.git/config"),
        &[other.to_owned()],
    );
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings);
    let days = scan(&mut sessions);
    let ids: Vec<&str> = days.iter().map(|day| day.id.as_str()).collect();
    assert_eq!(ids, ["s2"], "only the session of another repository");
}

#[test]
fn ais_024_a_home_with_no_session_folders_gives_nothing() {
    let home = tempfile::tempdir().expect("a home");
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, UsageSettings::default());
    assert_eq!(scan(&mut sessions).len(), 0);
}

#[test]
fn ais_024_only_a_daemon_of_a_user_reads_sessions() {
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    };
    assert_eq!(
        user_home(env(&[("HOME", "/home/me")])),
        Some("/home/me".into())
    );
    let system = env(&[
        ("HOME", "/var/lib/anima-daemon"),
        ("ANIMA_DAEMON_HOME", "/var/lib/anima-daemon"),
    ]);
    assert_eq!(user_home(system), None, "the system unit");
    assert_eq!(user_home(env(&[("HOME", "/var/lib/anima-daemon")])), None);
    assert_eq!(
        user_home(env(&[
            ("ANIMA_DAEMON_HOME", "/var/lib/anima-daemon"),
            ("HOME", "/home/me")
        ])),
        None
    );
    assert_eq!(user_home(env(&[])), None, "no HOME");
    assert_eq!(user_home(env(&[("HOME", "relative")])), None);
}

/// Counts read before the file names its folder are not sent: a later line may name an
/// excluded folder, and the hub would keep what it had (AIS-023, AU-5 security review).
#[test]
fn ais_023_nothing_of_a_file_is_sent_before_its_folder_is_known() {
    let home = tempfile::tempdir().expect("a home");
    let file = home.path().join(".claude/projects/-p/s9.jsonl");
    let no_folder = r#"{"type":"assistant","timestamp":"2026-11-08T09:00:00Z","message":{"model":"m","usage":{"output_tokens":3}}}"#;
    write(&file, &[no_folder.to_owned()]);
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings(home.path()));
    assert_eq!(scan(&mut sessions).len(), 0, "no folder named yet");
    append(&file, &claude_line(&home.path().join("private/app"), 1));
    assert_eq!(scan(&mut sessions).len(), 0, "the folder is excluded");
}

/// A session recorded through a link to an excluded folder is excluded (AIS-008, AU-5
/// security review).
#[test]
fn ais_008_a_working_folder_reached_through_a_link_is_excluded() {
    let home = tempfile::tempdir().expect("a home");
    let private = home.path().join("real/private");
    fs::create_dir_all(private.join("app")).expect("a folder");
    std::os::unix::fs::symlink(home.path().join("real"), home.path().join("via")).expect("a link");
    let recorded = home.path().join("via/private/app");
    write(
        &home.path().join(".claude/projects/-via/s8.jsonl"),
        &[claude_line(&recorded, 2)],
    );
    let settings = UsageSettings {
        excluded_folders: vec![fs::canonicalize(&private).expect("real")],
        ..UsageSettings::default()
    };
    let mut sessions = AgentSessions::new(home.path().to_owned(), false, settings);
    assert_eq!(scan(&mut sessions).len(), 0);
}
