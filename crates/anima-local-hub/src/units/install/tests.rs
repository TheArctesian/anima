use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use anima_database::Platform;
use tempfile::TempDir;

use super::{InstallError, Request, install, is_installed, remove};
use crate::programs::tests::carried;
use crate::{Carried, Folders, run};

mod start_stop;

struct Rig {
    folder: TempDir,
    folders: Folders,
    source: Carried,
}

fn rig() -> Rig {
    let folder = tempfile::tempdir().unwrap();
    let folders = Folders::new(folder.path().join("home/hub"), folder.path().join("s")).unwrap();
    let source = carried(folder.path(), "one");
    Rig {
        folder,
        folders,
        source,
    }
}

impl Rig {
    fn request(&self, platform: Platform) -> Request<'_> {
        Request {
            platform,
            uid: nix::unistd::Uid::effective().as_raw(),
            home: Box::leak(self.folder.path().join("home").into_boxed_path()),
            folders: &self.folders,
            listen: None,
            stop_limit: Duration::from_millis(300),
            step_back: false,
            version: "1.0.0",
        }
    }

    fn install(&self, platform: Platform) -> (Result<(), InstallError>, Vec<String>) {
        let mut ran = Vec::new();
        let outcome = install(&self.request(platform), &self.source, |step| {
            ran.push(format!("{} {}", step.program, step.args.join(" ")));
            Ok(true)
        });
        (outcome.map(|_| ()), ran)
    }

    fn remove(&self, platform: Platform) -> Vec<String> {
        let mut ran = Vec::new();
        remove(&self.request(platform), |step| {
            ran.push(format!("{} {}", step.program, step.args.join(" ")));
            Ok(true)
        })
        .unwrap();
        ran
    }

    fn home(&self, file: &str) -> std::path::PathBuf {
        self.folder.path().join("home").join(file)
    }
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn on_a_mac_install_stops_the_old_agent_writes_the_file_and_loads_it() {
    let rig = rig();
    let uid = nix::unistd::Uid::effective();
    let file = rig.home("Library/LaunchAgents/ing.anima.hub.plist");

    let (outcome, ran) = rig.install(Platform::MacOs);

    outcome.unwrap();
    assert_eq!(
        ran,
        [
            format!("/bin/launchctl bootout gui/{uid}/ing.anima.hub"),
            format!("/bin/launchctl bootstrap gui/{uid} {}", file.display()),
        ]
    );
    let text = fs::read_to_string(&file).unwrap();
    let program = rig.folders.programs().join("bin/anima-local-hub");
    assert!(
        text.contains(&format!("<string>{}</string>", program.display())),
        "{text}"
    );
    assert!(program.is_file());
    assert_eq!(mode_of(&file), 0o644);
    assert!(is_installed(&rig.request(Platform::MacOs)));
}

#[test]
fn on_linux_install_writes_the_user_unit_enables_it_and_starts_it() {
    let rig = rig();

    let (outcome, ran) = rig.install(Platform::Linux);

    outcome.unwrap();
    assert_eq!(
        ran,
        [
            "/usr/bin/systemctl --user stop anima-hub.service",
            "/usr/bin/systemctl --user daemon-reload",
            "/usr/bin/systemctl --user enable anima-hub.service",
            "/usr/bin/systemctl --user restart anima-hub.service",
        ]
    );
    let unit = fs::read_to_string(rig.home(".config/systemd/user/anima-hub.service")).unwrap();
    assert!(unit.contains("\"--socket-folder\""), "{unit}");
}

#[test]
fn a_second_install_is_an_update_and_leaves_no_file_of_the_work() {
    let rig = rig();
    rig.install(Platform::MacOs).0.unwrap();
    let newer = carried(rig.folder.path(), "two");

    install(&rig.request(Platform::MacOs), &newer, |_| Ok(true)).unwrap();

    let hub = rig.folders.programs().join("bin/anima-hub");
    assert_eq!(fs::read_to_string(hub).unwrap(), "two");
    let agents: Vec<_> = fs::read_dir(rig.home("Library/LaunchAgents"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(agents, ["ing.anima.hub.plist"]);
}

#[test]
fn dbl_013_the_programs_are_not_replaced_while_the_hub_still_runs() {
    let rig = rig();
    rig.install(Platform::MacOs).0.unwrap();
    let _held = run::lock(&rig.folders).unwrap();
    let newer = carried(rig.folder.path(), "two");

    let outcome = install(&rig.request(Platform::MacOs), &newer, |_| Ok(true));

    assert!(
        matches!(outcome, Err(InstallError::StillRunning)),
        "{outcome:?}"
    );
    let hub = rig.folders.programs().join("bin/anima-hub");
    assert_eq!(fs::read_to_string(hub).unwrap(), "one");
}

#[test]
fn a_bundle_that_cannot_be_copied_leaves_the_running_hub_alone() {
    let rig = rig();
    fs::remove_file(rig.source.program("anima-hub")).unwrap();

    let (outcome, ran) = rig.install(Platform::MacOs);

    assert!(matches!(outcome, Err(InstallError::Copy(_))), "{outcome:?}");
    assert_eq!(ran, Vec::<String>::new());
}

#[test]
fn a_command_that_fails_is_an_error_that_names_it() {
    let rig = rig();

    let outcome = install(&rig.request(Platform::Linux), &rig.source, |step| {
        Ok(!step.args.contains(&"enable".to_owned()))
    });

    let error = outcome.unwrap_err();
    assert_eq!(
        error.to_string(),
        "`/usr/bin/systemctl --user enable anima-hub.service` failed"
    );
}

#[test]
fn a_unit_folder_that_others_can_change_is_refused() {
    let rig = rig();
    let agents = rig.home("Library/LaunchAgents");
    fs::create_dir_all(&agents).unwrap();
    fs::set_permissions(&agents, fs::Permissions::from_mode(0o777)).unwrap();

    let (outcome, _ran) = rig.install(Platform::MacOs);

    assert!(
        matches!(outcome, Err(InstallError::OpenToOthers(_))),
        "{outcome:?}"
    );
    assert_eq!(fs::read_dir(&agents).unwrap().count(), 0);
}

#[test]
fn remove_stops_the_unit_and_takes_the_file_and_the_programs_but_never_the_database() {
    for (platform, file, commands) in [
        (
            Platform::MacOs,
            "Library/LaunchAgents/ing.anima.hub.plist",
            vec![format!(
                "/bin/launchctl bootout gui/{}/ing.anima.hub",
                nix::unistd::Uid::effective()
            )],
        ),
        (
            Platform::Linux,
            ".config/systemd/user/anima-hub.service",
            vec![
                "/usr/bin/systemctl --user disable --now anima-hub.service".to_owned(),
                "/usr/bin/systemctl --user daemon-reload".to_owned(),
            ],
        ),
    ] {
        let rig = rig();
        rig.install(platform).0.unwrap();
        fs::create_dir_all(rig.folders.database().cluster()).unwrap();
        fs::write(rig.folders.vault_key(), "kept").unwrap();

        let ran = rig.remove(platform);

        assert_eq!(ran, commands);
        assert!(!rig.home(file).exists());
        assert!(!rig.folders.all_programs().exists());
        assert!(!is_installed(&rig.request(platform)));
        assert!(rig.folders.database().cluster().is_dir());
        assert!(rig.folders.vault_key().is_file());
        // A second remove finds nothing and is not an error.
        rig.remove(platform);
    }
}

#[test]
fn a_hub_folder_below_a_folder_that_others_can_change_is_refused_before_anything_is_copied() {
    let rig = rig();
    let home = rig.home("");
    fs::create_dir_all(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o777)).unwrap();

    let (outcome, ran) = rig.install(Platform::MacOs);

    assert!(
        matches!(outcome, Err(InstallError::OpenToOthers(_))),
        "{outcome:?}"
    );
    assert_eq!(ran, Vec::<String>::new());
    assert!(!rig.folders.all_programs().exists());
    fs::set_permissions(&home, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn install_closes_a_hub_folder_that_was_open_to_others() {
    let rig = rig();
    fs::create_dir_all(rig.folders.root()).unwrap();
    fs::set_permissions(rig.folders.root(), fs::Permissions::from_mode(0o777)).unwrap();

    rig.install(Platform::MacOs).0.unwrap();

    assert_eq!(mode_of(rig.folders.root()), 0o700);
}

#[test]
fn a_unit_that_is_not_loaded_cannot_be_stopped_and_that_is_not_an_error() {
    let rig = rig();

    let outcome = install(&rig.request(Platform::MacOs), &rig.source, |step| {
        Ok(!step.args.contains(&"bootout".to_owned()))
    });

    outcome.unwrap();
    assert!(is_installed(&rig.request(Platform::MacOs)));
}
