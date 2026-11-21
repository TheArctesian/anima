//! Who may get the token at a socket of the link (K8 Section 1, HMD-054): a program that listens
//! at the path and is not the peer that the daemon expects gets no byte of a request, and on macOS
//! no folder that the environment names gets one.

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt as _;

use anima_daemon::config::private_file::running_user;
use anima_daemon::hub::answer::{Answer, Cause};
use anima_daemon::hub::peer::{Expected, Program};
use anima_daemon::hub::{HubClient, TIMEOUT, link_admits};
use daemon_core::wire::{Announcements, Batch, DaemonToken};
use tokio::io::AsyncReadExt as _;
use tokio::net::UnixListener;

/// A program at the socket, in a folder that only this user can reach.
pub fn listener_at_the_socket() -> (tempfile::TempDir, std::path::PathBuf, UnixListener) {
    let folder = tempfile::tempdir().expect("a folder");
    std::fs::set_permissions(folder.path(), Permissions::from_mode(0o700)).expect("mode");
    let socket = folder.path().join("link.sock");
    let listener = UnixListener::bind(&socket).expect("a socket");
    (folder, socket, listener)
}

/// Pushes with a token to the socket; returns the cause of the failure (`None` for an answer)
/// and each byte that the listener got on its one connection.
async fn push_to(listener: UnixListener, client: HubClient) -> (Option<Cause>, Vec<u8>) {
    let receiving = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("a connection");
        let mut got = Vec::new();
        stream.read_to_end(&mut got).await.expect("read");
        got
    });
    let token = DaemonToken::from_random([5; 32]);
    let batch = Batch::new(uuid::Uuid::from_u128(1), [], Announcements::default());
    let failed = match client.push(&token, &batch).await {
        Answer::Failed(cause) => Some(cause),
        Answer::Accepted(_) | Answer::Refused(_) => None,
    };
    drop(client);
    let got = tokio::time::timeout(TIMEOUT, receiving)
        .await
        .expect("the connection ends")
        .expect("the listener");
    (failed, got)
}

#[tokio::test]
async fn k8_1_a_listener_of_a_different_user_gets_no_byte_of_the_request() {
    let (_folder, socket, listener) = listener_at_the_socket();
    let someone_else = Expected {
        user: running_user().expect("the user") + 1,
        program: Program::OfTheUser,
    };
    let client = HubClient::over_link_to(&socket, TIMEOUT, someone_else).expect("a client");
    let (failed, got) = push_to(listener, client).await;
    assert_eq!(failed, Some(Cause::NotTheLink));
    assert!(got.is_empty(), "the listener got {} bytes", got.len());
}

/// The real `codesign` reads the signature of this test process, which has no Developer ID
/// signature, as a program of the same user that listens at the path would have none.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn k8_1_a_signed_daemon_sends_nothing_to_a_program_of_its_user_without_the_signature() {
    use anima_daemon::hub::peer::TeamId;
    let (_folder, socket, listener) = listener_at_the_socket();
    let signed = Expected {
        user: running_user().expect("the user"),
        program: Program::SignedBy(TeamId::parse("ABCDE12345").expect("a team")),
    };
    let client = HubClient::over_link_to(&socket, TIMEOUT, signed).expect("a client");
    let (failed, got) = push_to(listener, client).await;
    assert_eq!(failed, Some(Cause::NotTheLink));
    assert!(got.is_empty(), "the listener got {} bytes", got.len());
}

/// The finding `daemonoldlinkhome` of the review HM-T23: on macOS the token is in the Keychain,
/// so a daemon with the old link origin, whose socket is in a folder that `ANIMA_DAEMON_HOME`
/// can name, makes no client.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn k8_1_on_macos_a_daemon_with_the_old_link_origin_makes_no_client() {
    use anima_daemon::config::origin::Origin;
    use anima_daemon::hub::HubError;
    let old = Origin::parse_kept("http://127.0.0.1:4100").expect("a stored old link origin");
    let (_folder, socket, _listener) = listener_at_the_socket();
    assert!(matches!(
        HubClient::reaching(old.reach(), &socket, TIMEOUT),
        Err(HubError::OldLinkOrigin)
    ));
}

/// R-NEW-socketpeer: the move of an old enrolment asks only this check, which writes nothing.
#[tokio::test]
async fn k8_1_the_check_before_a_move_admits_the_peer_of_this_user_and_no_other() {
    let (_folder, socket, listener) = listener_at_the_socket();
    let me = running_user().expect("the user");
    let of_this_user = Expected {
        user: me,
        program: Program::OfTheUser,
    };
    let someone_else = Expected {
        user: me + 1,
        program: Program::OfTheUser,
    };
    assert!(link_admits(&socket, &of_this_user).await.expect("a check"));
    assert!(!link_admits(&socket, &someone_else).await.expect("a check"));
    drop(listener);
    std::fs::remove_file(&socket).expect("no socket");
    assert!(
        !link_admits(&socket, &of_this_user).await.expect("a check"),
        "nothing listens"
    );
}
