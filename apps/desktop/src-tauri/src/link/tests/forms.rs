use std::{ffi::OsString, path::Path};

use super::super::{
    LinkError, LinkStatus, environment, folder, from_main, printed,
    printed::DeviceLine,
    state::{DeviceState, Event, Phase, Reason},
};

const INVITE: &str = "anima-invite.1.q83vEjRWeJCrze8SNFZ4kA.hub-Address_1.relay-2";

#[test]
fn only_the_main_web_view_may_ask() {
    assert_eq!(from_main("main"), Ok(()));
    for other in ["web-1", "builder", ""] {
        assert_eq!(from_main(other), Err(LinkError::Caller), "{other:?}");
    }
}

#[test]
fn an_invite_is_one_line_of_the_invite_alphabet() {
    assert_eq!(&*printed::invite(INVITE).unwrap(), &format!("{INVITE}\n"));
    assert_eq!(
        &*printed::invite(&format!("  {INVITE}\r\n")).unwrap(),
        &format!("{INVITE}\n")
    );
    let too_long = format!("{INVITE}{}", "a".repeat(4096));
    for bad in [
        "",
        "anima-invite.1.",
        "anima-invite.2.abc",
        "https://evil.example/anima-invite.1.abc",
        "anima-invite.1.abc def",
        "anima-invite.1.abc\nsecond",
        "anima-invite.1.abc/def",
        "anima-invite.1.ab+c=",
        "anima-invite.1.abç",
        &too_long,
    ] {
        assert_eq!(
            printed::invite(bad).err(),
            Some(LinkError::BadInvite),
            "{bad:?}"
        );
    }
}

#[test]
fn the_fingerprint_is_read_from_the_join_answer_only() {
    let answer = "This device has joined the hub. Its key fingerprint is 1a2b3c4d.\n\
                  Start the link with: anima-link device -state /x\n";
    assert_eq!(printed::fingerprint(answer).as_deref(), Some("1a2b3c4d"));
    for bad in [
        "",
        "This device has joined the hub. Its key fingerprint is 1a2b.\n",
        "This device has joined the hub. Its key fingerprint is 1a2b3c4dz.\n",
        "This device has joined the hub. Its key fingerprint is 1a2b3c4d\n",
    ] {
        assert_eq!(printed::fingerprint(bad), None, "{bad:?}");
    }
}

#[test]
fn a_failed_join_is_no_answer_refused_or_failed() {
    assert_eq!(
        printed::join_failure(
            "anima-link: devicelink: the hub did not answer; the invite is used, expired or wrong, or the hub is off\n"
        ),
        LinkError::NoAnswer
    );
    assert_eq!(
        printed::join_failure(
            "anima-link: devicelink: the hub refused this device; ask for a new invite\n"
        ),
        LinkError::Refused
    );
    assert_eq!(
        printed::join_failure("anima-link: statedir: other users can reach this path\n"),
        LinkError::Failed
    );
}

#[test]
fn the_ready_line_gives_the_loopback_port_and_a_taken_port_is_seen() {
    let ready = "time=2026-11-15T10:00:00Z level=INFO msg=\"the device link is ready\" local=127.0.0.1:43123 key=1a2b3c4d\n";
    assert_eq!(printed::device_line(ready), Some(DeviceLine::Ready(43123)));
    assert_eq!(
        printed::device_line(
            "anima-link: listen tcp 127.0.0.1:43123: bind: address already in use"
        ),
        Some(DeviceLine::PortTaken)
    );
    for nothing in [
        "level=INFO msg=\"the device link stopped\"",
        "msg=\"the device link is ready\" local=10.0.0.1:43123",
        "msg=\"the device link is ready\" local=127.0.0.1:0",
        "msg=\"the device link is ready\"",
        "local=127.0.0.1:43123",
    ] {
        assert_eq!(printed::device_line(nothing), None, "{nothing:?}");
    }
}

#[test]
fn the_link_path_comes_from_a_device_status_only() {
    let status =
        br#"{"role":"device","local":"127.0.0.1:43123","path":"relayed","latencyMs":41.5}"#;
    assert_eq!(
        printed::link_path(status),
        Some(("relayed".into(), 41.5, None))
    );
    for bad in [
        &br#"{"role":"hub","path":"direct","latencyMs":1}"#[..],
        br#"{"role":"device","path":"<script>","latencyMs":1}"#,
        br#"{"role":"device","path":"direct","latencyMs":-1}"#,
        br#"{"role":"device","path":"direct"}"#,
        b"nonsense",
    ] {
        assert_eq!(printed::link_path(bad), None);
    }
}

#[test]
fn the_port_file_holds_one_port() {
    assert_eq!(folder::parse_port(b"43123\n"), Some(43123));
    assert_eq!(folder::parse_port(b"1"), Some(1));
    for bad in [
        &b""[..],
        b"0",
        b"65536",
        b"+80",
        b" 80",
        b"80\n\n",
        b"123456",
        b"8o",
    ] {
        assert_eq!(folder::parse_port(bad), None, "{bad:?}");
    }
}

#[test]
fn a_taken_port_is_reported_and_kept() {
    let state = DeviceState::default()
        .after(Event::Starting { port: 43123 })
        .after(Event::PortTaken)
        .after(Event::Ended);

    assert_eq!(state.phase, Phase::Down);
    assert_eq!(state.reason, Some(Reason::PortTaken));
    assert_eq!(state.port, Some(43123));
    assert!(!state.serves(43123));
}

#[test]
fn the_link_serves_its_port_only_while_up_on_the_reported_port() {
    let starting = DeviceState::default().after(Event::Starting { port: 43123 });
    assert!(!starting.serves(43123));

    let up = starting.after(Event::Ready { port: 43123 });
    assert_eq!(up.phase, Phase::Up);
    assert!(up.serves(43123));
    assert!(!up.serves(8443));

    let ended = up.after(Event::Ended);
    assert_eq!(
        (ended.phase, ended.reason),
        (Phase::Down, Some(Reason::Ended))
    );
    assert!(!ended.serves(43123));
    // A ready line after the end is not believed.
    assert!(!ended.after(Event::Ready { port: 43123 }).serves(43123));

    let stopped = up.after(Event::Stopped);
    assert_eq!((stopped.phase, stopped.reason), (Phase::Stopped, None));
}

#[test]
fn the_status_names_its_fields_for_the_client() {
    let status = LinkStatus {
        joined: true,
        state: Phase::Down,
        port: Some(43123),
        public_key: None,
        reason: Some(Reason::PortTaken),
        path: None,
        latency_ms: None,
    };
    assert_eq!(
        serde_json::to_value(&status).unwrap(),
        serde_json::json!({"joined": true, "state": "down", "port": 43123, "reason": "port-taken"})
    );
    let up = LinkStatus {
        state: Phase::Up,
        reason: None,
        path: Some("direct".into()),
        latency_ms: Some(2.0),
        ..status
    };
    assert_eq!(serde_json::to_value(&up).unwrap()["latencyMs"], 2.0);
}

#[test]
fn the_link_sees_only_home_and_tmpdir() {
    let env = environment(|name| match name {
        "HOME" => Some(OsString::from("/Users/ada")),
        "TMPDIR" => Some(OsString::new()),
        _ => Some(OsString::from("x")),
    });
    assert_eq!(env, [("HOME", OsString::from("/Users/ada"))]);
}

#[test]
fn the_device_folder_is_below_the_application_folder() {
    let folders = folder::LinkFolders::below(Path::new("/home/ada/.local/share/anima"));
    assert_eq!(
        folders.device,
        Path::new("/home/ada/.local/share/anima/link/device")
    );
    assert_eq!(
        folders.next,
        Path::new("/home/ada/.local/share/anima/link/device.next")
    );
}

#[test]
fn each_error_has_its_own_code() {
    let mut codes: Vec<&str> = LinkError::ALL.iter().map(|error| error.code()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), LinkError::ALL.len());
    let sent = serde_json::to_value(LinkError::NoAnswer).unwrap();
    assert_eq!(sent["code"], "no-answer");
}
