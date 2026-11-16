//! `start` and `stop` (FRN-006): the unit stays installed, and a stop waits for the hub.

use std::io;

use anima_database::Platform;

use super::super::{InstallError, start, stop};
use super::{Rig, rig};
use crate::run;
use crate::units::Step;

/// Runs `command` with each step answered by `answer`, and returns the steps.
fn steps(
    command: impl FnOnce(&mut dyn FnMut(&Step) -> io::Result<bool>) -> Result<(), InstallError>,
    answer: impl Fn(&Step) -> bool,
) -> (Result<(), InstallError>, Vec<String>) {
    let mut ran = Vec::new();
    let outcome = command(&mut |step| {
        ran.push(format!("{} {}", step.program, step.args.join(" ")));
        Ok(answer(step))
    });
    (outcome, ran)
}

fn installed(platform: Platform) -> Rig {
    let rig = rig();
    rig.install(platform).0.unwrap();
    rig
}

#[test]
fn frn_006_stop_unloads_the_agent_and_keeps_it_installed() {
    let uid = nix::unistd::Uid::effective();
    for (platform, command) in [
        (
            Platform::MacOs,
            format!("/bin/launchctl bootout gui/{uid}/ing.anima.hub"),
        ),
        (
            Platform::Linux,
            "/usr/bin/systemctl --user stop anima-hub.service".to_owned(),
        ),
    ] {
        let rig = installed(platform);
        let request = rig.request(platform);

        // A unit that is not loaded is stopped already: the failure is no error.
        let (outcome, ran) = steps(|run| stop(&request, run), |_| false);

        outcome.unwrap();
        assert_eq!(ran, [command]);
        assert!(super::is_installed(&request));
    }
}

#[test]
fn frn_006_stop_waits_for_the_hub_and_tells_when_it_does_not_stop() {
    let rig = installed(Platform::MacOs);
    let _held = run::lock(&rig.folders).unwrap();

    let (outcome, _) = steps(|run| stop(&rig.request(Platform::MacOs), run), |_| true);

    assert!(matches!(outcome, Err(InstallError::StillRunning)));
}

#[test]
fn frn_006_start_on_a_mac_loads_the_agent_unless_launchd_has_it() {
    let rig = installed(Platform::MacOs);
    let request = rig.request(Platform::MacOs);
    let uid = nix::unistd::Uid::effective();
    let file = rig.home("Library/LaunchAgents/ing.anima.hub.plist");
    let is_print = |step: &Step| step.args.first().is_some_and(|verb| verb == "print");

    let (outcome, ran) = steps(|run| start(&request, run), |step| !is_print(step));
    outcome.unwrap();
    assert_eq!(
        ran,
        [
            format!("/bin/launchctl print gui/{uid}/ing.anima.hub"),
            format!("/bin/launchctl bootstrap gui/{uid} {}", file.display()),
        ]
    );

    let (outcome, ran) = steps(|run| start(&request, run), |_| true);
    outcome.unwrap();
    assert_eq!(ran.len(), 1, "a loaded agent is left as it is: {ran:?}");
}

#[test]
fn frn_006_start_on_linux_starts_the_enabled_unit() {
    let rig = installed(Platform::Linux);

    let (outcome, ran) = steps(|run| start(&rig.request(Platform::Linux), run), |_| true);

    outcome.unwrap();
    assert_eq!(ran, ["/usr/bin/systemctl --user start anima-hub.service"]);
}

#[test]
fn start_without_an_installed_hub_runs_nothing() {
    let rig = rig();

    let (outcome, ran) = steps(|run| start(&rig.request(Platform::MacOs), run), |_| true);

    assert!(matches!(outcome, Err(InstallError::NotInstalled)));
    assert_eq!(ran, Vec::<String>::new());
}
