use std::fs;
use std::path::Path;

use super::*;

const CLAUDE: &str = concat!(
    r#"{"type":"user","timestamp":"2026-11-20T09:00:00Z","cwd":"/w/anima","gitBranch":"dev","message":{"content":"Fix the login test"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-11-20T09:00:02Z","message":{"content":[{"type":"text","text":"Fixed."}]}}"#,
    "\n"
);

const CODEX: &str = concat!(
    r#"{"timestamp":"2026-11-19T10:00:00Z","type":"session_meta","payload":{"id":"019a7c2e-codex","cwd":"/w/site"}}"#,
    "\n",
    r#"{"timestamp":"2026-11-19T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"Run the tests"}}"#,
    "\n"
);

fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("a home");
    let project = home.path().join(".claude/projects/-w-anima");
    fs::create_dir_all(project.join("4f1d2c3b-claude/subagents")).expect("folders");
    fs::write(project.join("4f1d2c3b-claude.jsonl"), CLAUDE).expect("written");
    fs::write(
        project.join("4f1d2c3b-claude/subagents/agent-1.jsonl"),
        CLAUDE,
    )
    .expect("written");
    fs::write(home.path().join(".claude/settings.json"), "{}").expect("written");
    let day = home.path().join(".codex/sessions/2026/11/19");
    fs::create_dir_all(&day).expect("folders");
    fs::write(
        day.join("rollout-2026-11-19T10-00-00-019a7c2e-codex.jsonl"),
        CODEX,
    )
    .expect("written");
    home
}

fn ids(rows: &[ThreadRow]) -> Vec<&str> {
    rows.iter().map(|row| row.id.as_str()).collect()
}

#[test]
fn thr_007_the_list_holds_the_sessions_of_both_programs_and_no_sub_agent() {
    let home = home();
    let rows = list(home.path(), None, DEFAULT_LIMIT);
    let mut found = ids(&rows);
    found.sort_unstable();
    assert_eq!(found, ["019a7c2e-codex", "4f1d2c3b-claude"]);
    let claude = rows
        .iter()
        .find(|row| row.program == "claude_code")
        .expect("a row");
    assert_eq!(claude.title.as_deref(), Some("Fix the login test"));
    assert_eq!(claude.folder.as_deref(), Some("/w/anima"));
}

#[test]
fn thr_007_a_program_and_a_limit_narrow_the_list() {
    let home = home();
    assert_eq!(
        ids(&list(home.path(), Some(Source::Codex), 10)),
        ["019a7c2e-codex"]
    );
    assert_eq!(list(home.path(), None, 1).len(), 1);
}

#[test]
fn thr_007_a_thread_is_found_by_its_id_or_the_end_of_its_file_name() {
    let home = home();
    assert_eq!(
        find(home.path(), "4f1d2c3b-claude").expect("found").source,
        Source::ClaudeCode
    );
    assert_eq!(
        find(home.path(), "019a7c2e-codex").expect("found").source,
        Source::Codex
    );
    assert!(matches!(
        find(home.path(), "short"),
        Err(Failure::Usage { .. })
    ));
    assert!(matches!(
        find(home.path(), "../../x/y"),
        Err(Failure::Usage { .. })
    ));
    assert!(matches!(
        find(home.path(), "00000000-none"),
        Err(Failure::NotFound { .. })
    ));
}

#[cfg(unix)]
#[test]
fn thr_007_a_session_file_that_is_a_link_is_not_listed() {
    let home = home();
    let project = home.path().join(".claude/projects/-w-anima");
    let outside = home.path().join("outside.jsonl");
    fs::write(&outside, CLAUDE).expect("written");
    std::os::unix::fs::symlink(&outside, project.join("9e9e9e9e-link.jsonl")).expect("a link");
    assert!(!ids(&list(home.path(), None, DEFAULT_LIMIT)).contains(&"9e9e9e9e-link"));
}

#[test]
fn thr_008_a_thread_reads_whole_and_shows_as_text() {
    let home = home();
    let file = find(home.path(), "4f1d2c3b-claude").expect("found");
    let thread = files::read(&file).expect("read");
    let text = shown::text(&ThreadRow::of(&file, thread.clone()), &thread);
    assert!(
        text.starts_with("Fix the login test · Claude Code\n/w/anima · dev\n"),
        "{text}"
    );
    assert!(text.contains("> You\n  Fix the login test\n"));
    assert!(text.contains("\n  Fixed.\n"));
    let json = serde_json::to_value(shown::Shown::of(&file, &thread)).expect("json");
    assert_eq!(json["entries"][0]["kind"], "prompt");
    assert_eq!(json["program"], "claude_code");
}

#[test]
fn thr_007_a_glance_reads_the_title_from_the_end_of_a_long_file() {
    let home = home();
    let file = home
        .path()
        .join(".claude/projects/-w-anima/7a7a7a7a-long.jsonl");
    let filler = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"x"}]}}"#;
    let mut text = CLAUDE.to_owned();
    while (text.len() as u64) < 400 * 1024 {
        text.push_str(filler);
        text.push('\n');
    }
    text.push_str(r#"{"type":"custom-title","customTitle":"Named late"}"#);
    text.push('\n');
    fs::write(&file, text).expect("written");
    let row = list(home.path(), None, DEFAULT_LIMIT)
        .into_iter()
        .find(|row| row.id == "7a7a7a7a-long")
        .expect("a row");
    assert_eq!(row.title.as_deref(), Some("Named late"));
    assert!(Path::new(&file).exists());
}

#[cfg(unix)]
#[test]
fn thr_007_a_session_file_with_a_second_name_is_not_read() {
    let home = home();
    let project = home.path().join(".claude/projects/-w-anima");
    let outside = home.path().join("outside.jsonl");
    fs::write(&outside, CLAUDE).expect("written");
    fs::hard_link(&outside, project.join("8b8b8b8b-hard.jsonl")).expect("a hard link");
    assert!(!ids(&list(home.path(), None, DEFAULT_LIMIT)).contains(&"8b8b8b8b-hard"));
}

#[cfg(unix)]
#[test]
fn thr_007_a_root_folder_that_is_a_link_gives_no_thread() {
    let home = home();
    let real = home.path().join("elsewhere");
    fs::rename(home.path().join(".codex"), &real).expect("moved");
    std::os::unix::fs::symlink(&real, home.path().join(".codex")).expect("a link");
    assert_eq!(
        ids(&list(home.path(), None, DEFAULT_LIMIT)),
        ["4f1d2c3b-claude"]
    );
}

#[test]
fn thr_007_the_id_of_a_row_is_the_id_that_opens_it() {
    let home = home();
    let day = home.path().join(".codex/sessions/2026/11/19");
    let forged = CODEX.replace("019a7c2e-codex", "4f1d2c3b-claude");
    fs::write(
        day.join("rollout-2026-11-19T11-00-00-5c5c5c5c-other.jsonl"),
        forged,
    )
    .expect("written");
    let rows = list(home.path(), Some(Source::Codex), DEFAULT_LIMIT);
    let row = rows
        .iter()
        .find(|row| row.id.ends_with("5c5c5c5c-other"))
        .expect("named by its file");
    assert_eq!(
        find(home.path(), &row.id).expect("found").source,
        Source::Codex
    );
}
