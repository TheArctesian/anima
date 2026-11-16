//! The socket client against a fake control socket: the request and reply lines, the 1 MiB
//! cap, the time limit and the peer's user (FRN-009, LNK-003).

use std::{fs, os::unix::fs::PermissionsExt as _};

use tokio::{net::UnixStream, time::Instant};

use super::{INVITE, fake, uid};
use crate::fleet::link::{
    Error,
    model::{ListedDevice, MadeInvite},
    socket::{CALL_LIMIT, Control, peer_is},
    wire::{MAX_LINE, Request},
};

#[tokio::test]
async fn lnk_003_the_client_sends_the_request_and_reads_the_invite() {
    let (_folder, path) = fake(|line| {
        assert_eq!(line, r#"{"args":{"name":"phone"},"verb":"invites.make"}"#);
        format!(
            r#"{{"ok":true,"result":{{"invite":"{INVITE}","expires":"2030-05-06T12:00:00Z"}}}}"#
        )
        .into_bytes()
        .into_iter()
        .chain(*b"\n")
        .collect()
    });
    let made: MadeInvite = Control::new(path, uid())
        .call(Request::InvitesMake { name: "phone" })
        .await
        .unwrap();
    assert_eq!(made.invite.expose(), INVITE);
}

#[tokio::test]
async fn frn_009_a_reply_over_one_mebibyte_is_refused() {
    let (_folder, path) = fake(|_| {
        let mut long = br#"{"ok":true,"result":""#.to_vec();
        long.resize(MAX_LINE + 10, b'a');
        long
    });
    let error = Control::new(path, uid())
        .call::<Vec<ListedDevice>>(Request::DevicesList)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Unreadable), "{error:?}");
}

#[tokio::test(start_paused = true)]
async fn frn_009_a_link_that_does_not_answer_times_out() {
    let (_folder, path) = fake(|_| Vec::new());
    let started = Instant::now();
    let error = Control::new(path, uid())
        .call::<Vec<ListedDevice>>(Request::DevicesList)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::TimedOut), "{error:?}");
    // At the limit, not later: the clock is paused, so no other time passes.
    assert_eq!(started.elapsed().as_secs(), CALL_LIMIT.as_secs());
}

#[tokio::test]
async fn frn_009_a_missing_socket_is_a_link_that_does_not_run() {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let error = Control::new(folder.path().join("control.sock"), uid())
        .call::<Vec<ListedDevice>>(Request::DevicesList)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::NotRunning), "{error:?}");
}

#[tokio::test]
async fn frn_009_a_reply_longer_than_the_first_buffer_is_read_whole() {
    let device = r#"{"key":"k","fingerprint":"f","name":"n","address":"fd41:4e49:4d41::7","expires":"2030-08-03T12:00:00Z","streams":1,"path":"none"}"#;
    let reply = format!(
        "{{\"ok\":true,\"result\":[{}]}}\n",
        vec![device; 3000].join(",")
    );
    assert!(reply.len() > 64 * 1024 && reply.len() < MAX_LINE);
    let (_folder, path) = fake(move |_| reply.clone().into_bytes());
    let devices: Vec<ListedDevice> = Control::new(path, uid())
        .call(Request::DevicesList)
        .await
        .unwrap();
    assert_eq!(devices.len(), 3000);
}

#[tokio::test]
async fn frn_009_a_process_of_another_user_at_the_socket_is_refused() {
    let (_folder, path) = fake(|_| Vec::new());
    let stream = UnixStream::connect(&path).await.unwrap();
    assert!(peer_is(&stream, uid()).is_ok());
    let error = peer_is(&stream, uid() + 1).unwrap_err();
    assert!(matches!(error, Error::Exposed(_)), "{error:?}");
}
