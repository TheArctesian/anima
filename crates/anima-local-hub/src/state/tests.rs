use std::time::{Duration, UNIX_EPOCH};

use super::{Blocked, Link, LinkState, State, Status, read, write};

/// 2027-01-15T08:00:00Z and a part of a second.
fn link(state: Link) -> LinkState {
    LinkState::at(state, UNIX_EPOCH + Duration::from_millis(1_800_000_000_250))
}

#[test]
fn a_state_that_was_written_is_read_back() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("state");
    let states = [
        State::Starting,
        State::Ready {
            hub: "127.0.0.1:8443".parse().unwrap(),
            recovered: true,
            link: Some(link(Link::Down)),
        },
        State::Blocked {
            reason: Blocked::KeyMissing,
        },
        State::Failed {
            cause: "anima-hub ended".to_owned(),
        },
    ];

    for state in states {
        write(&file, &state).unwrap();

        assert_eq!(read(&file), Some(state));
    }
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 1);
}

#[test]
fn a_ready_state_from_before_the_link_is_still_read() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("state");
    std::fs::write(
        &file,
        r#"{"state":"ready","hub":"127.0.0.1:8443","recovered":false}"#,
    )
    .unwrap();

    assert_eq!(
        read(&file),
        Some(State::Ready {
            hub: "127.0.0.1:8443".parse().unwrap(),
            recovered: false,
            link: None,
        })
    );
}

#[test]
fn no_file_and_a_file_that_is_not_a_state_are_no_state() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("state");

    assert_eq!(read(&file), None);
    std::fs::write(&file, "{\"state\":\"unheard-of\"}").unwrap();
    assert_eq!(read(&file), None);
}

#[test]
fn the_status_is_one_flat_object_with_stable_names() {
    let ready = Status {
        installed: true,
        running: true,
        last: Some(State::Ready {
            hub: "127.0.0.1:8443".parse().unwrap(),
            recovered: false,
            link: Some(link(Link::Up)),
        }),
    };
    let blocked = Status {
        installed: true,
        running: true,
        last: Some(State::Blocked {
            reason: Blocked::DatabaseVersion,
        }),
    };
    let never_ran = Status {
        installed: false,
        running: false,
        last: None,
    };

    assert_eq!(
        serde_json::to_string(&ready).unwrap(),
        r#"{"installed":true,"running":true,"state":"ready","hub":"127.0.0.1:8443","recovered":false,"link":{"state":"up","since":"2027-01-15T08:00:00Z"}}"#
    );
    assert_eq!(
        serde_json::to_string(&blocked).unwrap(),
        r#"{"installed":true,"running":true,"state":"blocked","reason":"database-version"}"#
    );
    assert_eq!(
        serde_json::to_string(&never_ran).unwrap(),
        r#"{"installed":false,"running":false}"#
    );
}
