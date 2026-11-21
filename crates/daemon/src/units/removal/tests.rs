//! The removal with a temporary HOME and a runner that only notes each command: no real unit,
//! `launchctl`, `systemctl` or Keychain is touched.

use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::*;

const UID: u32 = 4242;

/// What happened, in order: each command as one line, then `forget` when it was called.
#[derive(Default)]
struct Noted {
    lines: RefCell<Vec<String>>,
}

impl Noted {
    fn note(&self, line: String) {
        self.lines.borrow_mut().push(line);
    }

    fn lines(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }
}

fn env_of(home: &Path, runtime: Option<&str>) -> impl Fn(&str) -> Option<String> {
    let home = home.display().to_string();
    let runtime = runtime.map(str::to_owned);
    move |name| match name {
        "HOME" => Some(home.clone()),
        "XDG_RUNTIME_DIR" => runtime.clone(),
        _ => None,
    }
}

/// Runs the removal for `platform` in `home`; each command answers `answer`. The file of the
/// unit is noted as there or gone each time a command runs.
fn remove(
    platform: Platform,
    home: &Path,
    runtime: Option<&str>,
    answer: impl Fn(&Step) -> io::Result<bool>,
) -> (Result<Uninstalled, UninstallError>, Vec<String>) {
    let noted = Noted::default();
    let file = unit_file(platform, home);
    let result = uninstall(
        Leaving { platform, uid: UID },
        env_of(home, runtime),
        |step| {
            let state = if file.exists() {
                "file there"
            } else {
                "file gone"
            };
            let bus = step.env.iter().map(|(name, _)| *name).collect::<Vec<_>>();
            noted.note(format!(
                "{} {} ({state}; {})",
                step.program,
                step.args.join(" "),
                bus.join(",")
            ));
            answer(step)
        },
        || {
            noted.note("forget".to_owned());
            Ok(())
        },
    );
    (result, noted.lines())
}

fn unit_file(platform: Platform, home: &Path) -> PathBuf {
    match platform {
        Platform::Macos => home.join("Library/LaunchAgents/ing.anima.daemon.plist"),
        Platform::Linux => home.join(".config/systemd/user/anima-daemon.service"),
    }
}

fn installed(platform: Platform, home: &Path) -> PathBuf {
    let file = unit_file(platform, home);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "unit").unwrap();
    file
}

#[test]
fn hmd_056_a_launch_agent_is_booted_out_then_its_file_deleted_then_the_token_forgotten() {
    let home = tempfile::tempdir().unwrap();
    let file = installed(Platform::Macos, home.path());
    let (result, lines) = remove(Platform::Macos, home.path(), None, |_| Ok(true));
    let done = result.unwrap();
    assert_eq!(
        done,
        Uninstalled {
            file: file.clone(),
            was_installed: true
        }
    );
    assert!(!file.exists());
    assert_eq!(
        lines,
        [
            format!("/bin/launchctl bootout gui/{UID}/ing.anima.daemon (file there; )"),
            "forget".to_owned(),
        ]
    );
}

#[test]
fn hmd_056_a_second_removal_and_a_daemon_that_is_not_loaded_end_the_same() {
    let home = tempfile::tempdir().unwrap();
    // `bootout` of a daemon that is not loaded exits with a failure: that is fine.
    let (result, lines) = remove(Platform::Macos, home.path(), None, |_| Ok(false));
    assert!(!result.unwrap().was_installed);
    assert_eq!(lines.last().map(String::as_str), Some("forget"));
}

#[test]
fn hmd_056_a_systemd_user_unit_is_stopped_deleted_and_the_manager_reloaded_with_the_bus() {
    let home = tempfile::tempdir().unwrap();
    let file = installed(Platform::Linux, home.path());
    let (result, lines) = remove(Platform::Linux, home.path(), Some("/run/user/4242"), |_| {
        Ok(true)
    });
    assert!(result.unwrap().was_installed);
    assert!(!file.exists());
    assert_eq!(
        lines,
        [
            "/usr/bin/systemctl --user disable --now anima-daemon.service (file there; XDG_RUNTIME_DIR)",
            "/usr/bin/systemctl --user daemon-reload (file gone; XDG_RUNTIME_DIR)",
            "forget",
        ]
    );
}

#[test]
fn hmd_056_with_no_session_bus_a_missing_unit_still_forgets_and_a_present_one_is_an_error() {
    let home = tempfile::tempdir().unwrap();
    let (result, lines) = remove(Platform::Linux, home.path(), None, |_| Ok(true));
    // A uid with no /run/user folder of its own: no bus, and no unit to stop.
    if !Path::new("/run/user/4242").exists() {
        assert!(!result.unwrap().was_installed);
        assert_eq!(lines, ["forget"]);

        let file = installed(Platform::Linux, home.path());
        let (result, lines) = remove(Platform::Linux, home.path(), None, |_| Ok(true));
        assert!(matches!(result, Err(UninstallError::NoSessionBus(_))));
        assert!(lines.is_empty(), "nothing ran and nothing was forgotten");
        assert!(file.exists());
    }
}

#[test]
fn hmd_056_a_stop_that_cannot_run_keeps_the_unit_and_the_token() {
    let home = tempfile::tempdir().unwrap();
    let file = installed(Platform::Macos, home.path());
    let (result, lines) = remove(Platform::Macos, home.path(), None, |_| {
        Err(io::Error::from(io::ErrorKind::TimedOut))
    });
    assert!(matches!(
        result,
        Err(UninstallError::Run {
            program: "/bin/launchctl",
            ..
        })
    ));
    assert!(file.exists());
    assert!(!lines.contains(&"forget".to_owned()));
}

#[test]
fn hmd_056_a_failed_reload_after_the_deletion_is_an_error_and_keeps_the_token() {
    let home = tempfile::tempdir().unwrap();
    installed(Platform::Linux, home.path());
    let (result, lines) = remove(
        Platform::Linux,
        home.path(),
        Some("/run/user/4242"),
        |step| Ok(!step.args.contains(&"daemon-reload".to_owned())),
    );
    assert!(matches!(result, Err(UninstallError::Step { .. })));
    assert!(!lines.contains(&"forget".to_owned()));
}

#[test]
fn hmd_056_an_error_of_the_token_store_is_returned_after_the_unit_is_out() {
    let home = tempfile::tempdir().unwrap();
    let file = installed(Platform::Macos, home.path());
    let result = uninstall(
        Leaving {
            platform: Platform::Macos,
            uid: UID,
        },
        env_of(home.path(), None),
        |_| Ok(true),
        || Err(UninstallError::Token(TokenStoreError::Corrupt)),
    );
    assert!(matches!(result, Err(UninstallError::Token(_))));
    assert!(!file.exists());
}

#[test]
fn hmd_056_with_no_home_nothing_is_run() {
    let result = uninstall(
        Leaving {
            platform: Platform::Macos,
            uid: UID,
        },
        |_| None,
        |_| panic!("no command runs"),
        || panic!("nothing is forgotten"),
    );
    assert!(matches!(
        result,
        Err(UninstallError::Plan(PlanError::NoHome))
    ));
}
