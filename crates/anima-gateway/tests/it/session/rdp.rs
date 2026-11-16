//! A whole RDP session through `serve`, against the RDP device of the tests: the picture,
//! keys and the pointer, and each way the session ends.

use anima_gateway::rdp::{Button, KeyEvent, PointerEvent};
use anima_gateway::screen::Area;
use anima_gateway::session::{Ended, Settings};
use guac_protocol::Instruction;

use super::draw_frame;
use super::hub_side::{session, values};
use crate::drawn::Drawn;
use crate::rdp::device::{Device, Offered, Seen, Setup};

const PASSWORD: &str = "made up for the test";
const NLA: Offered = Offered::Nla {
    username: "operator",
    password: PASSWORD,
};

fn device(offered: Offered) -> Device {
    Device::start(Setup {
        size: (640, 480),
        offered,
    })
}

/// The accepted certificate as the hub sends it: `sha256:` and pairs of hex digits.
fn accepted(fingerprint: [u8; 32]) -> String {
    let pairs: Vec<_> = fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("sha256:{}", pairs.join(":"))
}

fn status(error: &Instruction) -> &str {
    &error.args[1]
}

#[tokio::test]
async fn a_session_draws_the_device_and_carries_keys_and_the_pointer() {
    let mut device = device(Offered::Tls);
    let fingerprint = accepted(device.fingerprint);
    let (mut hub, _served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("password", PASSWORD),
        ("security", "tls"),
        ("cert-fingerprints", fingerprint.as_str()),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");

    let Seen::SignIn {
        username, password, ..
    } = device.next().await
    else {
        panic!("the sign-in comes first")
    };
    assert_eq!(
        (username.as_str(), password.as_str()),
        ("operator", PASSWORD)
    );
    // Num Lock on and Caps Lock off, before any key.
    assert_eq!(device.next().await, Seen::Locks(0x02));

    let area = Area {
        x: 40,
        y: 30,
        width: 200,
        height: 100,
    };
    device.paint(area, [200, 30, 90]).await;
    let mut client = Drawn::black(640, 480);
    let painted = |client: &Drawn, x: usize, y: usize| {
        let at = (y * client.width + x) * 3;
        client.rgb[at..at + 3] == [200, 30, 90]
    };
    while !(painted(&client, 40, 30) && painted(&client, 239, 129)) {
        draw_frame(&mut hub, &mut client).await;
    }
    assert!(!painted(&client, 39, 30) && !painted(&client, 240, 130));

    hub.send(&Instruction::new("key", ["97", "1"])).await;
    let pressed = KeyEvent::Scancode {
        code: 0x1e,
        extended: false,
        pressed: true,
    };
    assert_eq!(device.next().await, Seen::Key(pressed));
    hub.send(&Instruction::new("mouse", ["100", "50", "1"]))
        .await;
    let at = |event| Seen::Pointer {
        x: 100,
        y: 50,
        event,
    };
    assert_eq!(device.next().await, at(PointerEvent::Move));
    let left = PointerEvent::Button {
        button: Button::Left,
        pressed: true,
    };
    assert_eq!(device.next().await, at(left));
}

#[tokio::test]
async fn a_device_with_another_certificate_gets_no_credential_and_the_session_ends() {
    let mut device = device(Offered::Tls);
    let other = accepted([7; 32]);
    let (mut hub, served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("password", PASSWORD),
        ("security", "tls"),
        ("cert-fingerprints", other.as_str()),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");

    // An upstream error: the hub ends the session `GATEWAY_ERROR` (K4 Section 6).
    assert_eq!(status(&hub.next_of("error").await), "515");
    assert_eq!(served.await.expect("served"), Ended::OtherCertificate);
    assert_eq!(device.next().await, Seen::Ended);
}

#[tokio::test]
async fn a_refused_nla_sign_in_ends_the_session_as_unauthorized() {
    let device = device(NLA);
    let (mut hub, served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("password", "not the password"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");

    assert_eq!(status(&hub.next_of("error").await), "769");
    assert_eq!(served.await.expect("served"), Ended::SignInRefused);
}

#[tokio::test]
async fn a_missing_password_is_asked_for_before_an_nla_sign_in() {
    let device = device(NLA);
    let (mut hub, _served) = session(Settings::default());
    let account = [("username", "operator"), ("ignore-cert", "true")];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");

    assert_eq!(hub.next_of("required").await.args, ["password"]);
    hub.send(&Instruction::new("argv", ["3", "text/plain", "password"]))
        .await;
    hub.send(&Instruction::new(
        "blob",
        ["3".to_owned(), super::base64(PASSWORD.as_bytes())],
    ))
    .await;
    hub.send(&Instruction::new("end", ["3"])).await;

    // The session opens: the device's size comes as the first drawing.
    assert_eq!(hub.next_of("size").await.args, ["0", "640", "480"]);
}

#[tokio::test]
async fn a_device_that_requires_nla_ends_a_tls_session_as_an_upstream_error() {
    let device = device(NLA);
    let (mut hub, served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("security", "tls"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");

    assert_eq!(status(&hub.next_of("error").await), "515");
    assert_eq!(served.await.expect("served"), Ended::SecurityNotOffered);
}

#[tokio::test]
async fn a_device_that_changes_its_screen_gives_the_client_the_new_size() {
    let mut device = device(Offered::Tls);
    let (mut hub, _served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("security", "tls"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");
    assert_eq!(hub.next_of("size").await.args, ["0", "640", "480"]);

    device.resize(500, 400).await;

    assert_eq!(hub.next_of("size").await.args, ["0", "500", "400"]);
    // The session goes on at the new size.
    let area = Area {
        x: 0,
        y: 0,
        width: 500,
        height: 400,
    };
    device.paint(area, [10, 20, 30]).await;
    hub.next_of("sync").await;
}

#[tokio::test]
async fn a_device_that_closes_ends_the_session_with_disconnect() {
    let mut device = device(Offered::Tls);
    let (mut hub, served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("security", "tls"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");
    hub.next_of("size").await;

    device.close().await;

    hub.next_of("disconnect").await;
    assert_eq!(served.await.expect("served"), Ended::ByDevice);
}

#[tokio::test]
async fn every_kind_of_key_and_button_reaches_the_device_as_its_rdp_event() {
    let mut device = device(Offered::Tls);
    let (mut hub, _served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("security", "tls"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");
    assert!(matches!(device.next().await, Seen::SignIn { .. }));
    assert_eq!(device.next().await, Seen::Locks(0x02));

    // Delete, an extended key, down and up; then a character with no US key.
    for (keysym, pressed) in [("65535", "1"), ("65535", "0"), ("246", "1")] {
        hub.send(&Instruction::new("key", [keysym, pressed])).await;
    }
    let delete = |pressed| KeyEvent::Scancode {
        code: 0x53,
        extended: true,
        pressed,
    };
    assert_eq!(device.next().await, Seen::Key(delete(true)));
    assert_eq!(device.next().await, Seen::Key(delete(false)));
    let o_umlaut = KeyEvent::Unicode {
        unit: 0xf6,
        pressed: true,
    };
    assert_eq!(device.next().await, Seen::Key(o_umlaut));

    // The right button, then the middle one in its place, then a step of the wheel.
    for mask in ["4", "2", "10"] {
        hub.send(&Instruction::new("mouse", ["7", "9", mask])).await;
    }
    let mut events = Vec::new();
    while events.len() < 7 {
        let Seen::Pointer { x: 7, y: 9, event } = device.next().await else {
            panic!("a pointer event at 7, 9")
        };
        events.push(event);
    }
    let button = |button, pressed| PointerEvent::Button { button, pressed };
    assert_eq!(
        events,
        [
            PointerEvent::Move,
            button(Button::Right, true),
            PointerEvent::Move,
            button(Button::Middle, true),
            button(Button::Right, false),
            PointerEvent::Move,
            PointerEvent::Wheel(120),
        ]
    );
}

#[tokio::test]
async fn keys_and_buttons_that_are_down_are_released_when_the_device_changes_its_screen() {
    let mut device = device(Offered::Tls);
    let (mut hub, _served) = session(Settings::default());
    let account = [
        ("username", "operator"),
        ("security", "tls"),
        ("ignore-cert", "true"),
    ];
    hub.open("rdp", values(device.address, &account))
        .await
        .expect("ready");
    assert!(matches!(device.next().await, Seen::SignIn { .. }));
    assert_eq!(device.next().await, Seen::Locks(0x02));
    hub.send(&Instruction::new("key", ["65505", "1"])).await;
    hub.send(&Instruction::new("mouse", ["20", "30", "1"]))
        .await;
    for _ in 0..3 {
        device.next().await;
    }

    device.resize(500, 400).await;

    // The device may have missed a release while it changed: nothing stays down.
    let shift_up = KeyEvent::Scancode {
        code: 0x2a,
        extended: false,
        pressed: false,
    };
    assert_eq!(device.next().await, Seen::Key(shift_up));
    let left_up = PointerEvent::Button {
        button: Button::Left,
        pressed: false,
    };
    assert_eq!(
        device.next().await,
        Seen::Pointer {
            x: 20,
            y: 30,
            event: left_up
        }
    );
}
