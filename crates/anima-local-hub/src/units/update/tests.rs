//! `update` (D-6): the programs change only when the application carries others.

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anima_database::Platform;
use tempfile::TempDir;

use super::{Outcome, update};
use crate::programs::tests::carried;
use crate::units::install::InstallError;
use crate::units::install::{self, Request, is_installed};
use crate::{Carried, Folders};

struct Rig {
    folder: TempDir,
    folders: Folders,
    source: Carried,
    /// The version of the programs that the application carries.
    version: Cell<&'static str>,
}

fn rig() -> Rig {
    let folder = tempfile::tempdir().unwrap();
    let folders = Folders::new(folder.path().join("home/hub"), folder.path().join("s")).unwrap();
    let source = carried(folder.path(), "one");
    Rig {
        folder,
        folders,
        source,
        version: Cell::new("1.0.0"),
    }
}

impl Rig {
    fn request(&self) -> Request<'_> {
        self.request_on(Platform::MacOs)
    }

    fn request_on(&self, platform: Platform) -> Request<'_> {
        Request {
            platform,
            uid: nix::unistd::Uid::effective().as_raw(),
            home: Box::leak(self.folder.path().join("home").into_boxed_path()),
            folders: &self.folders,
            listen: None,
            stop_limit: Duration::from_millis(300),
            version: self.version.get(),
        }
    }

    fn install(&self) {
        install::install(&self.request(), &self.source, |_| Ok(true)).unwrap();
    }

    fn update(&self) -> (bool, Vec<String>) {
        let (outcome, ran) = self.outcome(Platform::MacOs, |_| true);
        (outcome.unwrap().replaced(), ran)
    }

    /// Updates with each step answered by `answer`.
    fn outcome(
        &self,
        platform: Platform,
        answer: impl Fn(&crate::units::Step) -> bool,
    ) -> (Result<Outcome, InstallError>, Vec<String>) {
        let mut ran = Vec::new();
        let outcome = update(&self.request_on(platform), &self.source, |step| {
            ran.push(format!("{} {}", step.program, step.args.join(" ")));
            Ok(answer(step))
        });
        (outcome, ran)
    }

    fn installed_program(&self) -> String {
        fs::read_to_string(self.folders.programs().join("bin/anima-hub")).unwrap()
    }

    fn data(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let root = self.folders.root();
        let files = [
            self.folders.database().root().join("cluster/PG_VERSION"),
            self.folders.vault_key(),
            self.folders.link().join("key"),
            self.folders.logs().join("local-hub.log"),
            self.folders.state_file(),
        ];
        files
            .into_iter()
            .map(|file| {
                assert!(file.starts_with(root));
                let bytes = fs::read(&file).unwrap();
                (file, bytes)
            })
            .collect()
    }

    fn make_data(&self) {
        for (file, bytes) in [
            (
                self.folders.database().root().join("cluster/PG_VERSION"),
                "17",
            ),
            (self.folders.vault_key(), "secret"),
            (self.folders.link().join("key"), "link key"),
            (self.folders.logs().join("local-hub.log"), "a line"),
            (self.folders.state_file(), "{}"),
        ] {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, bytes).unwrap();
        }
    }

    fn new_programs(&self) {
        fs::write(self.source.program("anima-hub"), "two").unwrap();
    }
}

/// What a test does to a rig, with what it is.
type Act = (&'static str, fn(&Rig));

mod guards;
mod listen;

const STOP: &str = "/bin/launchctl bootout";

fn is_stop_then_start(ran: &[String]) -> bool {
    matches!(ran, [stop, start] if stop.starts_with(STOP) && start.starts_with("/bin/launchctl bootstrap"))
}

#[test]
fn a_hub_that_is_not_installed_is_not_updated() {
    let rig = rig();

    let (updated, ran) = rig.update();

    assert!(!updated);
    assert_eq!(ran, Vec::<String>::new());
    assert!(!rig.folders.all_programs().exists());
    assert!(!is_installed(&rig.request()));
}

#[test]
fn a_copy_that_is_the_same_runs_no_step_and_stays_as_it_is() {
    let rig = rig();
    rig.install();
    let marker = rig.folders.programs().join("bin/kept");
    fs::write(&marker, "this file is not in the source").unwrap();
    // A hub that runs: nothing is to be started.
    let _runs = crate::run::lock(&rig.folders).unwrap();

    let (updated, ran) = rig.update();

    assert!(!updated);
    assert!(ran.is_empty(), "{ran:?}");
    assert!(marker.is_file(), "the copy was replaced");
}

#[test]
fn a_changed_set_is_put_in_place_with_a_stop_and_a_start() {
    let changes: [Act; 5] = [
        ("a program", Rig::new_programs),
        ("a tree file", |rig| {
            fs::write(rig.source.tree().join("share/timezone/UTC"), "two").unwrap();
        }),
        ("a link", |rig| {
            let link = rig.source.tree().join("lib/libpq.5.dylib");
            fs::remove_file(&link).unwrap();
            std::os::unix::fs::symlink("libpq.5.18.dylib", link).unwrap();
        }),
        ("an executable bit", |rig| {
            use std::os::unix::fs::PermissionsExt as _;
            let file = rig.source.tree().join("bin/pg_ctl");
            fs::set_permissions(file, fs::Permissions::from_mode(0o644)).unwrap();
        }),
        ("an added file", |rig| {
            fs::write(rig.source.tree().join("share/new"), "").unwrap();
        }),
    ];
    for (what, change) in changes {
        let rig = rig();
        rig.install();
        change(&rig);

        let (updated, ran) = rig.update();

        assert!(updated, "{what}");
        assert!(is_stop_then_start(&ran), "{what}: {ran:?}");
        let (again, ran) = rig.update();
        let replaced = ran
            .iter()
            .any(|step| step.contains("bootout") || step.contains("bootstrap"));
        assert!(
            !again && !replaced,
            "{what}: the copy is the source now: {ran:?}"
        );
    }
}

#[test]
fn an_update_replaces_the_programs_that_run() {
    let rig = rig();
    rig.install();
    rig.new_programs();

    rig.update();

    assert_eq!(rig.installed_program(), "two");
}

#[test]
fn a_hub_that_an_older_version_installed_has_no_digest_and_is_updated() {
    let rig = rig();
    rig.install();
    fs::remove_file(rig.folders.programs().join("digest")).unwrap();

    let (updated, ran) = rig.update();

    assert!(updated);
    assert!(is_stop_then_start(&ran), "{ran:?}");
    assert!(rig.folders.programs().join("digest").is_file());
}

#[test]
fn the_database_the_key_the_link_and_the_logs_are_not_touched_by_an_update() {
    let rig = rig();
    rig.install();
    rig.make_data();
    let before = rig.data();
    rig.new_programs();

    let (updated, _) = rig.update();

    assert!(updated);
    assert_eq!(rig.data(), before);
}

#[test]
fn a_hub_that_the_operator_stopped_gets_the_new_programs_and_stays_stopped() {
    let rig = rig();
    rig.install();
    install::stop(&rig.request(), |_| Ok(true)).unwrap();
    rig.new_programs();

    let (updated, ran) = rig.update();

    assert!(updated);
    assert_eq!(rig.installed_program(), "two");
    assert!(
        ran.iter().all(|step| step.starts_with(STOP)),
        "no start step: {ran:?}"
    );
    assert!(rig.folders.stopped_by_operator().exists());
}

#[test]
fn start_install_and_remove_take_the_operators_stop_away() {
    let rig = rig();
    rig.install();
    let mark = rig.folders.stopped_by_operator();
    let steps: [Act; 3] = [
        ("start", |rig| {
            install::start(&rig.request(), |_| Ok(true)).unwrap();
        }),
        ("install", Rig::install),
        ("remove", |rig| {
            install::remove(&rig.request(), |_| Ok(true)).unwrap();
        }),
    ];
    for (name, step) in steps {
        install::stop(&rig.request(), |_| Ok(true)).unwrap();
        assert!(mark.exists(), "stop wrote no mark before {name}");

        step(&rig);

        assert!(!mark.exists(), "{name} left the mark");
    }
}
