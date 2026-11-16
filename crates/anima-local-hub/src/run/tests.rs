use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::watch;

use super::{Ended, RunError, run};
use crate::child::Stop;
use crate::group::{Limits, Plan};
use crate::state::{self, Blocked, State};
use crate::{Carried, Folders};

mod stopping;

const SOON: Duration = Duration::from_secs(20);

/// Stand-ins for the three programs. Each writes what it does to `events`, and a file in the
/// rig's folder changes what it does.
const DATABASE: &str = r#"#!/bin/sh
here=$(dirname "$0")/..
trap 'echo "database stop" >> "$here/events"; exit 0' TERM
echo "database start" >> "$here/events"
[ -f "$here/database-exit" ] && exit "$(cat "$here/database-exit")"
mkdir -p "$5/cluster"
[ -f "$here/recovered" ] && echo "anima-database: recovered after an unclean stop" >&2
[ -f "$here/database-slow" ] && while :; do sleep 0.05; done
echo ready
while :; do sleep 0.05; done
"#;
const HUB: &str = r#"#!/bin/sh
here=$(dirname "$0")/..
if [ "$1" = migrate ]; then
  trap 'echo "migrate stop" >> "$here/events"; exit 1' TERM
  echo "migrate as $(basename "$ANIMA_DATABASE_URL_FILE")" >> "$here/events"
  [ -f "$here/migrate-fails" ] && exit 1
  [ -f "$here/migrate-slow" ] && while :; do sleep 0.05; done
  exit 0
fi
trap 'echo "hub stop" >> "$here/events"; exit 0' TERM
[ -f "$ANIMA_KEY_FILE" ] || exit 9
echo "hub start as $(basename "$ANIMA_DATABASE_URL_FILE") gateway $ANIMA_GUACD" >> "$here/events"
echo '{"fields":{"message":"hub started","listen":"'"$ANIMA_LISTEN"'"}}'
[ -f "$here/hub-ends" ] && { sleep 0.3; exit 7; }
while :; do sleep 0.05; done
"#;
const GATEWAY: &str = r#"#!/bin/sh
here=$(dirname "$0")/..
trap 'echo "gateway stop" >> "$here/events"; exit 0' TERM
echo "gateway start" >> "$here/events"
echo "anima-gateway: listening on 127.0.0.1:4999" >&2
while :; do sleep 0.05; done
"#;

struct Rig {
    folder: TempDir,
    plan: Plan,
}

fn rig() -> Rig {
    let folder = tempfile::tempdir().unwrap();
    let programs = folder.path().join("carried");
    fs::create_dir_all(programs.join("bin")).unwrap();
    for (name, script) in [
        ("anima-database", DATABASE),
        ("anima-hub", HUB),
        ("anima-gateway", GATEWAY),
    ] {
        let path = programs.join("bin").join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let limit = Duration::from_secs(10);
    let plan = Plan {
        carried: Carried::installed_in(&programs),
        folders: Folders::new(folder.path().join("hub root"), folder.path().join("s")).unwrap(),
        listen: "127.0.0.1:8443".parse().unwrap(),
        limits: Limits {
            database_ready: limit,
            migration: limit,
            gateway_ready: limit,
            hub_ready: limit,
            hub_stop: limit,
            gateway_stop: limit,
            database_stop: limit,
        },
    };
    Rig { folder, plan }
}

impl Rig {
    fn set(&self, switch: &str, text: &str) {
        fs::write(self.folder.path().join("carried").join(switch), text).unwrap();
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(self.folder.path().join("carried/events"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn state(&self) -> Option<State> {
        state::read(&self.plan.folders.state_file())
    }

    fn key(&self) -> PathBuf {
        self.plan.folders.vault_key()
    }

    /// Runs the group and asks it to stop as soon as it is ready.
    async fn run_until_ready(&self) -> Result<Ended, RunError> {
        let (ask, stop) = Stop::new();
        self.run_with(stop, move || {
            ask.send_replace(true);
        })
        .await
    }

    async fn run_with(&self, stop: Stop, ready: impl FnOnce()) -> Result<Ended, RunError> {
        tokio::time::timeout(SOON, run(&self.plan, stop, ready))
            .await
            .expect("the run ends")
    }

    /// Runs the group, waits until its state is `wanted`, then asks it to stop.
    async fn run_until_state(&self, wanted: &State) -> Result<Ended, RunError> {
        let (ask, stop) = Stop::new();
        let asking = async {
            while self.state().as_ref() != Some(wanted) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            ask.send_replace(true);
        };
        let (ended, ()) = tokio::join!(self.run_with(stop, || {}), asking);
        ended
    }
}

fn never_asked() -> (watch::Sender<bool>, Stop) {
    Stop::new()
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[tokio::test]
async fn dbl_012_the_group_starts_in_order_and_stops_in_the_reverse_order() {
    let rig = rig();

    let ended = rig.run_until_ready().await.unwrap();

    assert_eq!(ended, Ended::Asked);
    assert_eq!(
        rig.events(),
        [
            "database start",
            "migrate as owner.url",
            "gateway start",
            "hub start as hub.url gateway 127.0.0.1:4999",
            "hub stop",
            "gateway stop",
            "database stop",
        ]
    );
    assert_eq!(rig.state(), Some(State::Stopped));
}

#[tokio::test]
async fn a_new_hub_gets_its_key_before_the_database_is_made_and_keeps_its_folders_private() {
    let rig = rig();

    rig.run_until_ready().await.unwrap();

    assert_eq!(mode_of(&rig.key()), 0o600);
    assert_eq!(mode_of(rig.plan.folders.root()), 0o700);
    assert_eq!(mode_of(&rig.plan.folders.logs()), 0o700);
    assert_eq!(mode_of(&rig.plan.folders.state_file()), 0o600);
    let log = fs::read_to_string(rig.plan.folders.logs().join("anima-hub.log")).unwrap();
    assert!(log.contains("hub started"), "{log}");
}

#[tokio::test]
async fn the_ready_state_tells_where_the_hub_listens_and_that_the_database_recovered() {
    let rig = rig();
    rig.set("recovered", "");
    let ready = State::Ready {
        hub: "127.0.0.1:8443".parse().unwrap(),
        recovered: true,
    };

    let ended = rig.run_until_state(&ready).await.unwrap();

    assert_eq!(ended, Ended::Asked);
}

#[tokio::test]
async fn when_a_member_ends_the_others_are_stopped_in_order_and_the_run_fails() {
    let rig = rig();
    rig.set("hub-ends", "");
    let (_ask, stop) = never_asked();

    let ended = rig.run_with(stop, || {}).await.unwrap();

    assert_eq!(ended, Ended::Failed);
    assert_eq!(rig.events()[4..], ["gateway stop", "database stop"]);
    assert_eq!(
        rig.state(),
        Some(State::Failed {
            cause: "anima-hub ended".to_owned()
        })
    );
}

#[tokio::test]
async fn a_failed_migration_stops_the_database_and_never_starts_the_hub() {
    let rig = rig();
    rig.set("migrate-fails", "");
    let (_ask, stop) = never_asked();

    let ended = rig.run_with(stop, || {}).await.unwrap();

    assert_eq!(ended, Ended::Failed);
    assert_eq!(
        rig.events(),
        ["database start", "migrate as owner.url", "database stop"]
    );
    assert!(matches!(rig.state(), Some(State::Failed { .. })));
}

#[tokio::test]
async fn dbl_009_a_database_of_another_version_blocks_the_hub_until_the_run_is_stopped() {
    for (code, reason) in [
        ("11", Blocked::DatabaseVersion),
        ("10", Blocked::DatabaseInUse),
    ] {
        let rig = rig();
        rig.set("database-exit", code);

        let ended = rig
            .run_until_state(&State::Blocked { reason })
            .await
            .unwrap();

        // Not a failure: the unit must not start the group again in a loop.
        assert_eq!(ended, Ended::Asked);
        assert_eq!(rig.events(), ["database start"]);
    }
}

#[tokio::test]
async fn a_lost_key_blocks_the_hub_and_no_new_key_is_made() {
    let rig = rig();
    rig.run_until_ready().await.unwrap();
    fs::remove_file(rig.key()).unwrap();
    fs::remove_file(rig.folder.path().join("carried/events")).unwrap();
    let blocked = State::Blocked {
        reason: Blocked::KeyMissing,
    };

    let ended = rig.run_until_state(&blocked).await.unwrap();

    assert_eq!(ended, Ended::Asked);
    assert!(!rig.key().exists());
    assert_eq!(
        rig.events(),
        Vec::<String>::new(),
        "nothing of the group ran"
    );
}

#[tokio::test]
async fn no_key_is_made_for_a_database_that_cannot_be_looked_at() {
    let rig = rig();
    let database = rig.plan.folders.database().root().to_path_buf();
    fs::create_dir_all(database.join("cluster")).unwrap();
    fs::set_permissions(&database, fs::Permissions::from_mode(0o000)).unwrap();
    let (_ask, stop) = never_asked();

    let ended = rig.run_with(stop, || {}).await.unwrap();

    fs::set_permissions(&database, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(ended, Ended::Failed);
    assert!(!rig.key().exists());
    assert_eq!(rig.events(), Vec::<String>::new());
}

#[tokio::test]
async fn a_second_run_of_the_same_folder_is_refused_and_changes_nothing() {
    let rig = rig();
    let (ask, stop) = Stop::new();
    let first = rig.run_with(stop, || {});
    let second = async {
        while !super::is_running(&rig.plan.folders) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (_ask, stop) = never_asked();
        let refused = run(&rig.plan, stop, || {}).await;
        ask.send_replace(true);
        refused
    };

    let (first, second) = tokio::join!(first, second);

    assert!(matches!(second, Err(RunError::AnotherCopy)));
    assert_eq!(first.unwrap(), Ended::Asked);
    assert!(!super::is_running(&rig.plan.folders));
}
