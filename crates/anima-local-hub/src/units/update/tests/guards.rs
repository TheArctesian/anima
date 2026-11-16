//! What stops an update from harm: a failed start, an older set, two changes at once, a kill.

use std::fs;

use anima_database::Platform;

use super::super::Outcome;
use super::rig;
use crate::units::change_lock;
use crate::units::install::{self, InstallError, is_installed};

pub(super) fn is_print(step: &crate::units::Step) -> bool {
    step.args.first().is_some_and(|arg| arg == "print")
}

#[test]
fn a_start_that_failed_after_the_copy_was_in_place_is_tried_again_by_the_next_update() {
    let rig = rig();
    rig.install();
    rig.new_programs();
    let (failed, _) = rig.outcome(Platform::MacOs, |step| {
        step.args.first().is_none_or(|a| a != "bootstrap")
    });
    assert!(
        matches!(failed, Err(InstallError::Step { .. })),
        "{failed:?}"
    );

    // The unit is not loaded: `print` fails, so the start steps run.
    let (outcome, ran) = rig.outcome(Platform::MacOs, |step| !is_print(step));

    assert_eq!(outcome.unwrap(), Outcome::Unchanged);
    assert_eq!(ran.len(), 2, "{ran:?}");
    assert!(
        ran[0].contains("print") && ran[1].contains("bootstrap"),
        "{ran:?}"
    );
}

#[test]
fn no_start_is_tried_when_the_operator_stopped_the_hub_or_the_hub_runs() {
    let rig = rig();
    rig.install();

    let hub_runs = crate::run::lock(&rig.folders).unwrap();
    let (outcome, ran) = rig.outcome(Platform::MacOs, |step| !is_print(step));
    assert_eq!((outcome.unwrap(), ran), (Outcome::Unchanged, Vec::new()));
    drop(hub_runs);

    install::stop(&rig.request(), |_| Ok(true)).unwrap();
    let (outcome, ran) = rig.outcome(Platform::MacOs, |step| !is_print(step));
    assert_eq!((outcome.unwrap(), ran), (Outcome::Unchanged, Vec::new()));
}

#[test]
fn programs_of_a_lower_version_do_not_replace_the_programs_that_run() {
    let rig = rig();
    rig.version.set("0.10.0");
    rig.install();
    rig.new_programs();
    rig.version.set("0.9.0");

    let (outcome, ran) = rig.outcome(Platform::MacOs, |_| true);

    assert_eq!(
        outcome.unwrap(),
        Outcome::Older {
            carried: "0.9.0".into(),
            installed: "0.10.0".into()
        }
    );
    assert_eq!(ran, Vec::<String>::new());
    assert_eq!(rig.installed_program(), "one");
}

#[test]
fn programs_of_the_same_or_a_higher_version_with_another_digest_replace_them() {
    for version in ["1.0.0", "1.0.1", "2.0.0"] {
        let rig = rig();
        rig.install();
        rig.new_programs();
        rig.version.set(version);

        let (updated, _) = rig.update();

        assert!(updated, "{version}");
        assert_eq!(
            fs::read_to_string(rig.folders.programs().join("version")).unwrap(),
            format!("{version}\n")
        );
    }
}

#[test]
fn an_install_replaces_the_programs_also_with_a_lower_version() {
    let rig = rig();
    rig.version.set("2.0.0");
    rig.install();
    rig.new_programs();
    rig.version.set("1.0.0");

    rig.install();

    assert_eq!(rig.installed_program(), "two");
}

#[test]
fn a_copy_without_a_version_file_does_not_block_an_update() {
    let rig = rig();
    rig.install();
    fs::remove_file(rig.folders.programs().join("version")).unwrap();
    rig.new_programs();
    rig.version.set("0.0.1");

    assert!(rig.update().0);
}

#[test]
fn a_second_change_of_the_same_hub_at_the_same_time_is_refused() {
    let rig = rig();
    rig.install();
    let held = change_lock::take(&rig.folders).unwrap();
    assert!(matches!(
        change_lock::take(&rig.folders),
        Err(InstallError::Busy)
    ));
    rig.new_programs();

    let (outcome, ran) = rig.outcome(Platform::MacOs, |_| true);
    assert!(matches!(outcome, Err(InstallError::Busy)));
    let request = rig.request();
    let results = [
        install::install(&request, &rig.source, |_| Ok(true)).map(drop),
        install::remove(&request, |_| Ok(true)),
        install::start(&request, |_| Ok(true)),
        install::stop(&request, |_| Ok(true)),
    ];

    assert!(
        results
            .iter()
            .all(|result| matches!(result, Err(InstallError::Busy)))
    );
    assert_eq!(ran, Vec::<String>::new());
    assert_eq!(rig.installed_program(), "one");
    drop(held);
    assert!(rig.update().0);
}

#[test]
fn a_kill_between_the_two_moves_of_a_replacement_is_repaired_by_the_next_update() {
    let rig = rig();
    rig.install();
    let programs = rig.folders.programs();
    fs::rename(&programs, rig.folders.programs_replaced()).unwrap();
    fs::create_dir_all(rig.folders.programs_being_copied()).unwrap();
    assert!(!is_installed(&rig.request()));

    let (updated, _) = rig.update();
    assert!(!updated, "the same programs: only the repair");
    assert!(is_installed(&rig.request()));

    fs::rename(&programs, rig.folders.programs_replaced()).unwrap();
    rig.new_programs();

    assert!(rig.update().0);
    assert_eq!(rig.installed_program(), "two");
}

#[test]
fn on_linux_a_hub_that_the_operator_stopped_gets_a_stop_and_a_daemon_reload_and_no_start() {
    let rig = rig();
    install::install(&rig.request_on(Platform::Linux), &rig.source, |_| Ok(true)).unwrap();
    install::stop(&rig.request_on(Platform::Linux), |_| Ok(true)).unwrap();
    rig.new_programs();

    let (outcome, ran) = rig.outcome(Platform::Linux, |_| true);

    assert_eq!(outcome.unwrap(), Outcome::Replaced);
    assert_eq!(
        ran,
        [
            "/usr/bin/systemctl --user stop anima-hub.service",
            "/usr/bin/systemctl --user daemon-reload"
        ]
    );
}
