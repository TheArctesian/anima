//! The agents that are set up by their own program, and the check of the `anima` path.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use super::super::super::agents::{State, agent};
use super::super::super::{Asked, change, listing};
use super::super::{Action, CommandError, Step, check_command};
use super::{computer, script, state_of};

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
        ran.contains(&format!("mcp add anima -- {} mcp", anima.path().display())),
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
    // A server at a longer path is not this one.
    fs::write(
        here.home.join("codex.state"),
        format!("{}-other mcp\n", anima.path().display()),
    )
    .expect("entry");
    assert_eq!(
        state_of(&listing(&here, &anima, Action::Install), "codex"),
        State::Different
    );
    assert!(agent("codex").is_some() && agent("nobody").is_none());
}

#[test]
fn mcp_033_the_program_of_the_entry_is_absolute_runnable_plain_and_not_replaceable_by_others() {
    let (_home, here, checked) = computer();
    let anima = checked.path().to_path_buf();
    let mode = |path: &Path, mode: u32| {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("mode");
    };
    assert_eq!(
        check_command(Path::new("anima"), here.user),
        Err(CommandError::NotAbsolute)
    );
    assert_eq!(
        check_command(&here.home.join("bin/../bin/anima"), here.user),
        Err(CommandError::NotAbsolute)
    );
    assert_eq!(
        check_command(&here.home.join("bin/none"), here.user),
        Err(CommandError::NotAProgram)
    );
    mode(&anima, 0o644);
    assert_eq!(
        check_command(&anima, here.user),
        Err(CommandError::NotAProgram)
    );
    mode(&anima, 0o777);
    assert_eq!(
        check_command(&anima, here.user),
        Err(CommandError::Replaceable)
    );
    mode(&anima, 0o755);
    assert_eq!(
        check_command(&anima, here.user + 1),
        Err(CommandError::Replaceable)
    );
    // A folder above the program that all can write, and the folder of what a link points at.
    let shared = here.home.join("shared");
    fs::create_dir_all(shared.join("mine")).expect("folders");
    let inside = script(&shared.join("mine"), "anima", "exit 0");
    let link = here.home.join("bin/linked");
    std::os::unix::fs::symlink(&inside, &link).expect("link");
    assert!(check_command(&inside, here.user).is_ok() && check_command(&link, here.user).is_ok());
    mode(&shared, 0o777);
    assert_eq!(
        check_command(&inside, here.user),
        Err(CommandError::Replaceable)
    );
    assert_eq!(
        check_command(&link, here.user),
        Err(CommandError::Replaceable)
    );
    // A path that an agent would read as a variable.
    let odd = here.home.join("${workspaceFolder}");
    fs::create_dir(&odd).expect("folder");
    assert_eq!(
        check_command(&script(&odd, "anima", "exit 0"), here.user),
        Err(CommandError::NotAbsolute)
    );
}

#[test]
fn mcp_029_035_036_a_different_entry_of_an_own_command_is_shown_before_it_is_replaced_and_kept_on_uninstall()
 {
    let (_home, here, anima) = computer();
    let log = here.home.join("claude.log");
    let state = here.home.join("claude.state");
    fs::write(&state, "anima: /opt/other/anima mcp\n").expect("their entry");
    script(
        &here.path[0],
        "claude",
        &format!(
            "echo \"$PWD $@\" >> {log}\ncase \"$2\" in add) echo \"$9\" > {state};; remove) rm -f {state};; get) cat {state} 2>/dev/null || exit 1;; esac",
            log = log.display(),
            state = state.display()
        ),
    );
    let rows = listing(&here, &anima, Action::Install);
    let row = rows
        .iter()
        .find(|row| row.id == "claude-code")
        .expect("the agent");
    assert_eq!(row.state, State::Different);
    let shown: Vec<&str> = row
        .steps
        .iter()
        .map(|step| match step {
            Step::Run { arguments, .. } => arguments[1].as_str(),
            Step::Write { .. } => "write",
        })
        .collect();
    assert_eq!(shown, ["remove", "add"]);
    // Uninstall leaves the entry of a person.
    let _ = change(
        &here,
        &anima,
        &Asked {
            action: Action::Uninstall,
            ids: &[],
            force: false,
        },
    );
    assert!(state.exists());
    let rows = change(
        &here,
        &anima,
        &Asked {
            action: Action::Install,
            ids: &[],
            force: false,
        },
    );
    assert_eq!(state_of(&rows, "claude-code"), State::SetUp);
    // The agent's program ran in the home folder, not in the folder of the caller.
    let ran = fs::read_to_string(&log).expect("log");
    let home = fs::canonicalize(&here.home).expect("home");
    assert!(
        ran.lines()
            .all(|line| line.starts_with(&*home.to_string_lossy())
                || line.starts_with(&*here.home.to_string_lossy())),
        "{ran}"
    );
}
