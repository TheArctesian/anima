//! The corrections of the review of the Settings part (MI5, 12.7 of the plan): who can replace a
//! folder, and an agent's program that a different user can replace.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use super::super::super::agents::State;
use super::super::super::{Asked, change, listing};
use super::super::{Action, Part, open_to_others};
use super::{computer, script, state_of};

const USER: u32 = 501;

fn folder(owner: u32, group: u32, mode: u32) -> Part {
    Part {
        owner,
        group,
        mode,
        folder: true,
        link: false,
    }
}

#[test]
fn mcp_033_on_macos_a_folder_of_root_that_the_group_admin_can_write_is_fixed() {
    // `/Applications` as macOS makes it: root, group admin, mode 0775.
    let applications = folder(0, 80, 0o40775);
    assert!(!open_to_others(applications, USER, true));
    // Elsewhere the group 80 is an ordinary group: a different user in it can replace the folder.
    assert!(open_to_others(applications, USER, false));
    // The group admin does not make a folder of a different user fixed.
    assert!(open_to_others(folder(502, 80, 0o40775), USER, true));
    // Write for all users stays open, also for root.
    assert!(open_to_others(folder(0, 80, 0o40777), USER, true));
}

#[test]
fn mcp_033_a_folder_of_this_user_and_its_private_group_is_fixed_and_a_shared_group_is_not() {
    assert!(!open_to_others(folder(USER, USER, 0o40775), USER, false));
    assert!(open_to_others(folder(USER, 20, 0o40775), USER, false));
    assert!(!open_to_others(folder(USER, 20, 0o40755), USER, false));
}

#[test]
fn mcp_035_an_agent_program_that_a_different_user_can_replace_is_not_run() {
    let (_home, mut here, anima) = computer();
    // A folder of `PATH` that every user can write, with no sticky bit.
    let shared = here.home.join("shared-bin");
    fs::create_dir(&shared).expect("folder");
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).expect("mode");
    let ran = here.home.join("ran");
    script(&shared, "codex", &format!("touch {}", ran.display()));
    here.path.insert(0, shared);
    assert_eq!(
        state_of(&listing(&here, &anima, Action::Install), "codex"),
        State::Unreadable
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
    let problem = rows
        .iter()
        .find(|row| row.id == "codex")
        .and_then(|row| row.problem.clone())
        .unwrap_or_default();
    assert!(problem.contains("different user"), "{problem}");
    assert!(!ran.exists(), "the program ran");
}
