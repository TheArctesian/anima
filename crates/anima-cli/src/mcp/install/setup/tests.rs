use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use super::super::agents::{State, agent};
use super::super::files::copy_path;
use super::super::{Asked, change, listing};
use super::{Action, CommandError, Here, Step, check_command};

/// A temporary home with a folder of programs on `PATH` and an `anima` in it.
fn computer() -> (tempfile::TempDir, Here, PathBuf) {
    let home = tempfile::tempdir().expect("a temporary home");
    let user = fs::metadata(home.path()).expect("owner").uid();
    let bin = home.path().join("bin");
    fs::create_dir(&bin).expect("bin");
    let anima = script(&bin, "anima", "exit 0");
    let here = Here {
        home: home.path().to_path_buf(),
        path: vec![bin],
        user,
        macos: false,
    };
    (home, here, anima)
}

fn script(folder: &Path, name: &str, body: &str) -> PathBuf {
    let path = folder.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode");
    path
}

fn state_of(rows: &[super::super::Row], id: &str) -> State {
    rows.iter()
        .find(|row| row.id == id)
        .expect("the agent")
        .state
}

#[test]
fn mcp_027_an_agent_is_listed_as_not_on_this_computer_until_its_folder_or_its_program_is_there() {
    let (_home, here, anima) = computer();
    let rows = listing(&here, &anima, Action::Install);
    assert!(
        rows.iter()
            .all(|row| row.state == State::NotInstalled && row.step.is_none())
    );
    fs::create_dir(here.home.join(".cursor")).expect("folder");
    let rows = listing(&here, &anima, Action::Install);
    assert_eq!(state_of(&rows, "cursor"), State::NotSetUp);
    assert_eq!(
        rows.iter()
            .find(|row| row.id == "cursor")
            .and_then(|row| row.step.clone()),
        Some(Step::Write {
            file: here.home.join(".cursor/mcp.json"),
            copy: None
        })
    );
}

#[test]
fn mcp_028_030_032_install_changes_only_the_entry_keeps_a_copy_and_uninstall_gives_the_file_back() {
    let (_home, here, anima) = computer();
    fs::create_dir_all(here.home.join(".config/zed")).expect("folder");
    let file = here.home.join(".config/zed/settings.json");
    let before = "// my settings\n{\n  \"theme\": \"One Dark\", // keep\n  \"context_servers\": {\n    \"mine\": { \"command\": \"x\" },\n  },\n}\n";
    fs::write(&file, before).expect("file");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("mode");
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Install,
            ids: &["zed".to_owned()],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "zed"), State::SetUp);
    let after = fs::read_to_string(&file).expect("after");
    assert!(after.starts_with("// my settings\n{\n  \"theme\": \"One Dark\", // keep\n  \"context_servers\": {\n    \"mine\": { \"command\": \"x\" },\n    \"anima\": {"));
    assert!(after.contains(&format!("\"command\": \"{}\"", anima.display())));
    assert_eq!(fs::read_to_string(copy_path(&file)).expect("copy"), before);
    // A second install has no work and writes nothing.
    let stamp = fs::metadata(&file).expect("file").mtime_nsec();
    let _ = change(
        &here,
        &anima,
        &Asked {
            action: Action::Install,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(fs::metadata(&file).expect("file").mtime_nsec(), stamp);
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Uninstall,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "zed"), State::NotSetUp);
    assert_eq!(fs::read_to_string(&file).expect("back"), before);
}

#[test]
fn mcp_036_uninstall_leaves_an_entry_that_starts_a_different_program_unless_forced() {
    let (_home, here, anima) = computer();
    fs::create_dir(here.home.join(".cursor")).expect("folder");
    let file = here.home.join(".cursor/mcp.json");
    let theirs =
        "{\"mcpServers\":{\"anima\":{\"command\":\"/opt/other/anima\",\"args\":[\"mcp\"]}}}";
    fs::write(&file, theirs).expect("file");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("mode");
    assert_eq!(
        state_of(&listing(&here, &anima, Action::Uninstall), "cursor"),
        State::Different
    );
    let _ = change(
        &here,
        &anima,
        &Asked {
            action: Action::Uninstall,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(fs::read_to_string(&file).expect("kept"), theirs);
    let _ = change(
        &here,
        &anima,
        &Asked {
            action: Action::Uninstall,
            ids: &[],
            force: true,
        },
    );
    assert_eq!(
        fs::read_to_string(&file).expect("removed"),
        "{\"mcpServers\":{}}"
    );
}

#[test]
fn mcp_034_a_refused_file_is_named_and_not_changed_and_the_other_agents_go_on() {
    let (_home, here, anima) = computer();
    fs::create_dir(here.home.join(".cursor")).expect("folder");
    fs::create_dir(here.home.join(".copilot")).expect("folder");
    let elsewhere = here.home.join("elsewhere.json");
    fs::write(&elsewhere, "{}").expect("target");
    std::os::unix::fs::symlink(&elsewhere, here.home.join(".cursor/mcp.json")).expect("link");
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Install,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "cursor"), State::Unreadable);
    assert_eq!(fs::read_to_string(&elsewhere).expect("target"), "{}");
    assert_eq!(state_of(&rows, "vscode"), State::SetUp);
    let text = fs::read_to_string(here.home.join(".copilot/mcp-config.json")).expect("made");
    assert!(
        text.contains("\"type\": \"stdio\"") && !text.contains("env") && !text.contains("token")
    );
}

#[test]
fn mcp_035_an_agent_with_its_own_command_is_set_up_by_that_program_with_fixed_arguments() {
    let (_home, here, anima) = computer();
    let log = here.home.join("codex.log");
    let bin = here.path[0].clone();
    // A stand-in for the agent: it records its arguments, and `get` fails until `add` ran.
    script(
        &bin,
        "codex",
        &format!(
            "echo \"$@\" >> {log}\ncase \"$2\" in add) echo \"$5\" > {state};; remove) rm -f {state};; get) cat {state} 2>/dev/null || exit 1;; esac",
            log = log.display(),
            state = here.home.join("codex.state").display()
        ),
    );
    assert_eq!(
        state_of(&listing(&here, &anima, Action::Install), "codex"),
        State::NotSetUp
    );
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Install,
            ids: &["codex".to_owned()],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "codex"), State::SetUp);
    let ran = fs::read_to_string(&log).expect("log");
    assert!(
        ran.contains(&format!("mcp add anima -- {} mcp", anima.display())),
        "{ran}"
    );
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Uninstall,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "codex"), State::NotSetUp);
    assert!(agent("codex").is_some() && agent("nobody").is_none());
}

#[test]
fn mcp_033_the_program_of_the_entry_is_absolute_runnable_and_not_replaceable_by_others() {
    let (_home, here, anima) = computer();
    assert_eq!(check_command(&anima, here.user), Ok(()));
    assert_eq!(
        check_command(Path::new("anima"), here.user),
        Err(CommandError::NotAbsolute)
    );
    assert_eq!(
        check_command(&here.home.join("bin/none"), here.user),
        Err(CommandError::NotAProgram)
    );
    fs::set_permissions(&anima, fs::Permissions::from_mode(0o644)).expect("mode");
    assert_eq!(
        check_command(&anima, here.user),
        Err(CommandError::NotAProgram)
    );
    fs::set_permissions(&anima, fs::Permissions::from_mode(0o777)).expect("mode");
    assert_eq!(
        check_command(&anima, here.user),
        Err(CommandError::Replaceable)
    );
    fs::set_permissions(&anima, fs::Permissions::from_mode(0o755)).expect("mode");
    fs::set_permissions(here.home.join("bin"), fs::Permissions::from_mode(0o777)).expect("mode");
    assert_eq!(
        check_command(&anima, here.user),
        Err(CommandError::Replaceable)
    );
    assert_eq!(
        check_command(&anima, here.user + 1),
        Err(CommandError::Replaceable)
    );
}
