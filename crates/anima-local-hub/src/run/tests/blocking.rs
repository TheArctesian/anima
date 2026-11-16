//! A group that the operator must unblock, and the key rule (S4 Section 1.5).

use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::{never_asked, rig};
use crate::run::Ended;
use crate::state::{Blocked, State};

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
