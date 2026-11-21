//! The move of an old enrolment (`http://127.0.0.1:<port>`) to the socket of the link, with the
//! Linux token file (the macOS store is the Keychain, which no test touches).

use anima_daemon::config::Config;
use anima_daemon::config::home::Home;
use anima_daemon::config::origin::Origin;
use anima_daemon::config::private_file::running_user;
use anima_daemon::hub::peer::{Expected, Program};
use anima_daemon::runtime::old_origin::Enrolled;
use anima_daemon::token_store::{Credential, TokenStore};
use chrono::DateTime;
use daemon_core::wire::DaemonToken;
use uuid::Uuid;

use crate::peer::listener_at_the_socket;

fn old() -> Origin {
    Origin::parse_kept("http://127.0.0.1:4100").expect("an old link origin")
}

/// A daemon in a new folder, enrolled with the old link origin.
fn enrolled_before_the_socket() -> (tempfile::TempDir, Home, TokenStore, Enrolled) {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("daemon"));
    let config = Config {
        hub: old(),
        device: Uuid::from_u128(7),
        device_name: "this computer".to_owned(),
    };
    home.write_config(&config).expect("a configuration");
    let store = TokenStore::open(&home);
    let credential = Credential {
        token: DaemonToken::from_random([9; 32]),
        expires_at: DateTime::UNIX_EPOCH,
        hub: old(),
    };
    store.save_new(&credential).expect("a token record");
    let credential = store.load().expect("a record").expect("a credential");
    (dir, home, store, Enrolled { config, credential })
}

fn expecting(user: u32) -> Expected {
    Expected {
        user,
        program: Program::OfTheUser,
    }
}

#[tokio::test]
async fn an_old_enrolment_moves_to_the_socket_when_its_peer_passes_the_check() {
    let (_dir, home, store, enrolled) = enrolled_before_the_socket();
    let (_folder, socket, _listener) = listener_at_the_socket();
    let me = expecting(running_user().expect("the user"));

    let (moved, failed) = enrolled.moved(&home, &store, (socket.as_path(), &me)).await;
    assert!(failed.is_none(), "the move failed");

    let to = Origin::parse(&format!("unix:{}", socket.display())).expect("a link origin");
    assert_eq!(moved.config.hub, to);
    assert_eq!(home.read_config().expect("read").expect("a file").hub, to);
    let kept = store.load().expect("read").expect("a record");
    assert_eq!(kept.hub, to);
    assert_eq!(
        kept.token.expose(),
        DaemonToken::from_random([9; 32]).expose()
    );
}

#[tokio::test]
async fn an_old_enrolment_does_not_move_to_a_peer_that_fails_the_check() {
    let (_dir, home, store, enrolled) = enrolled_before_the_socket();
    let (_folder, socket, _listener) = listener_at_the_socket();
    let someone_else = expecting(running_user().expect("the user") + 1);

    let (stayed, failed) = enrolled
        .moved(&home, &store, (socket.as_path(), &someone_else))
        .await;
    assert!(failed.is_none(), "no move, no error");

    assert_eq!(stayed.config.hub, old());
    assert_eq!(
        home.read_config().expect("read").expect("a file").hub,
        old()
    );
    assert_eq!(store.load().expect("read").expect("a record").hub, old());
}
