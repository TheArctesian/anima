//! The link's wire form; the socket client and the ownership check against a fake control
//! socket are in `tests/` (FRN-009, LNK-003, LNK-004, LOG-007).

mod owned;
mod socket;

use std::{fs, os::unix::fs::PermissionsExt as _, path::PathBuf};

use proptest::prelude::*;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{UnixListener, UnixStream},
};

use super::{
    Error, LinkPath, Refusal,
    model::{ListedDevice, MadeInvite, Revoked},
    wire::{Request, parse_reply},
};

pub(super) const INVITE: &str = "anima-invite.1.SECRETSECRETSECRET";
const DEVICE: &str = r#"{"key":"nodekey:0a0b","fingerprint":"1a2b3c4d","name":"laptop",
    "address":"fd41:4e49:4d41::7","permitted":"2030-05-05T12:00:00.123456789Z",
    "expires":"2030-08-03T12:00:00.123456789Z","streams":2,"path":"relayed","relay":"derp-ams"}"#;

pub(super) fn uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

#[test]
fn frn_009_requests_name_their_verb_and_only_their_arguments() {
    let text = |request: Request<'_>| String::from_utf8(request.line()).unwrap();
    assert_eq!(text(Request::DevicesList), "{\"verb\":\"devices.list\"}\n");
    assert_eq!(
        text(Request::DevicesRevoke { key: "nodekey:0a" }),
        "{\"args\":{\"key\":\"nodekey:0a\"},\"verb\":\"devices.revoke\"}\n"
    );
    // A quote in a name stays inside its JSON string.
    assert_eq!(
        text(Request::InvitesMake { name: "a\"b" }),
        "{\"args\":{\"name\":\"a\\\"b\"},\"verb\":\"invites.make\"}\n"
    );
}

#[test]
fn lnk_004_a_device_line_of_the_link_is_read_with_each_field() {
    let line = format!(r#"{{"ok":true,"result":[{DEVICE}]}}"#);
    let devices: Vec<ListedDevice> = parse_reply(line.as_bytes()).unwrap();
    let device = &devices[0];
    assert_eq!(device.key, "nodekey:0a0b");
    assert_eq!(device.fingerprint, "1a2b3c4d");
    assert_eq!(device.address.to_string(), "fd41:4e49:4d41::7");
    assert_eq!(device.path, LinkPath::Relayed);
    assert_eq!(device.relay.as_deref(), Some("derp-ams"));
    assert_eq!(device.streams, 2);
    assert_eq!(
        device.expires.to_rfc3339(),
        "2030-08-03T12:00:00.123456789+00:00"
    );
    let direct = DEVICE.replace(
        r#""path":"relayed","relay":"derp-ams""#,
        r#""path":"direct""#,
    );
    let devices: Vec<ListedDevice> =
        parse_reply(format!(r#"{{"ok":true,"result":[{direct}]}}"#).as_bytes()).unwrap();
    assert_eq!(
        (devices[0].path, devices[0].relay.clone()),
        (LinkPath::Direct, None)
    );
}

#[test]
fn hmd_047_a_device_line_gives_when_the_link_last_saw_the_device_or_nothing() {
    let read = |device: String| {
        let line = format!(r#"{{"ok":true,"result":[{device}]}}"#);
        let devices: Vec<ListedDevice> = parse_reply(line.as_bytes()).unwrap();
        devices[0].seen.map(|seen| seen.to_rfc3339())
    };
    // An older link, and a device that the link never saw.
    assert_eq!(read(DEVICE.to_owned()), None);
    let seen = DEVICE.replace(
        r#""streams":2"#,
        r#""streams":2,"seen":"2030-05-06T07:08:09.5Z""#,
    );
    assert_eq!(read(seen).as_deref(), Some("2030-05-06T07:08:09.500+00:00"));
    let never = DEVICE.replace(r#""streams":2"#, r#""streams":2,"seen":null"#);
    assert_eq!(read(never), None);
}

#[test]
fn frn_009_a_refusal_is_typed_by_its_code_and_keeps_no_message() {
    for (code, refusal) in [
        ("device_unknown", Refusal::DeviceUnknown),
        ("bad_name", Refusal::BadName),
        ("bad_request", Refusal::BadRequest),
        ("failed", Refusal::Other),
    ] {
        let line = format!(r#"{{"ok":false,"error":{{"code":"{code}","message":"{INVITE}"}}}}"#);
        let error = parse_reply::<Revoked>(line.as_bytes()).unwrap_err();
        assert!(
            matches!(error, Error::Refused(found) if found == refusal),
            "{code}"
        );
        assert!(!format!("{error} {error:?}").contains(INVITE));
    }
}

#[test]
fn log_007_an_unreadable_reply_carries_none_of_its_text() {
    for line in [
        format!(r#"{{"ok":true,"result":{{"invite":"{INVITE}"}}}}"#),
        format!(r#"{{"ok":true,"result":{{"invite":"{INVITE}","expires":"x"}}}}"#),
        format!(r#"{{"ok":true,"result":{{"invite":"{INVITE}"}},"extra":1}}"#),
        format!(r#"{{"ok":false,"result":{{"invite":"{INVITE}"}}}}"#),
        format!(r#"{{"ok":true,"error":{{"code":"x"}},"result":"{INVITE}"}}"#),
    ] {
        let error = parse_reply::<MadeInvite>(line.as_bytes()).unwrap_err();
        assert!(matches!(error, Error::Unreadable), "{line}");
        assert!(!format!("{error} {error:?}").contains(INVITE));
    }
    let line = format!(
        r#"{{"ok":true,"result":{{"invite":"{INVITE}","expires":"2030-05-06T12:00:00Z"}}}}"#
    );
    let made: MadeInvite = parse_reply(line.as_bytes()).unwrap();
    assert_eq!(made.invite.expose(), INVITE);
    assert!(!format!("{made:?}").contains(INVITE));
}

proptest! {
    #[test]
    fn frn_009_any_reply_bytes_parse_or_fail_without_their_text(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        if let Err(error) = parse_reply::<Vec<ListedDevice>>(&bytes) {
            let text = String::from_utf8_lossy(&bytes);
            let said = format!("{error} {error:?}");
            prop_assert!(text.len() < 8 || !said.contains(text.as_ref()));
        }
    }

    #[test]
    fn frn_009_a_device_list_reply_round_trips_its_counts(streams in 0_i64..1_000_000, count in 0_usize..20) {
        let device = DEVICE.replace("\"streams\":2", &format!("\"streams\":{streams}"));
        let line = format!(r#"{{"ok":true,"result":[{}]}}"#, vec![device; count].join(","));
        let devices: Vec<ListedDevice> = parse_reply(line.as_bytes()).unwrap();
        prop_assert_eq!(devices.len(), count);
        prop_assert!(devices.iter().all(|device| device.streams == streams));
    }
}

#[test]
fn frn_009_the_invite_verbs_and_an_answer_with_no_result() {
    let text = |request: Request<'_>| String::from_utf8(request.line()).unwrap();
    assert_eq!(text(Request::InvitesList), "{\"verb\":\"invites.list\"}\n");
    assert_eq!(
        text(Request::InvitesCancel { id: "ab12" }),
        "{\"args\":{\"id\":\"ab12\"},\"verb\":\"invites.cancel\"}\n"
    );
    parse_reply::<()>(br#"{"ok":true,"result":null}"#).unwrap();
    let missing = parse_reply::<Vec<ListedDevice>>(br#"{"ok":true,"result":null}"#);
    assert!(matches!(missing, Err(Error::Unreadable)), "{missing:?}");
}

/// A private folder with a socket that answers each request line with `answer(line)`.
pub(super) fn fake(
    answer: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static,
) -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = folder.path().join("control.sock");
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let answer: Answer = std::sync::Arc::new(answer);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(stream, answer.clone()));
        }
    });
    (folder, path)
}

type Answer = std::sync::Arc<dyn Fn(&str) -> Vec<u8> + Send + Sync>;

async fn serve(stream: UnixStream, answer: Answer) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let _ = write.write_all(&answer(&line)).await;
    }
}
