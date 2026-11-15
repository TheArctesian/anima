//! A whole session through the gateway: the hub's handshake on one side, a VNC device on
//! the other (K4 Sections 3 to 5).

mod device;
mod hub_side;
mod program;

use std::time::Duration;

use anima_gateway::session::{Ended, Settings};
use base64::Engine as _;
use guac_protocol::Instruction;

use crate::drawn::Drawn;
use crate::vnc::rfb;
use device::{ALL, Device, SIZE, SignIn, Tell};
use hub_side::{HubSide, session, values};

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Draws the gateway's instructions until the `sync` of a frame, and answers it.
async fn draw_frame<S>(hub: &mut HubSide<S>, client: &mut Drawn)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        let instruction = hub.next().await.expect("a frame");
        let sync = instruction.opcode == "sync";
        client.receive(std::slice::from_ref(&instruction));
        if sync {
            hub.send(&instruction).await;
            return;
        }
    }
}

/// Waits until the device has heard `count` messages.
async fn heard(device: &Device, count: usize) -> Vec<String> {
    for _ in 0..500 {
        if device.heard().len() >= count {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    device.heard()
}

#[tokio::test]
async fn a_session_draws_the_device_and_carries_input_and_the_clipboard() {
    let device = Device::start(SignIn::Password { accepted: true }).await;
    let (mut hub, served) = session(Settings::default());
    let id = hub
        .open("vnc", values(device.address, &[("password", "secret12")]))
        .await
        .expect("ready");
    assert_eq!(id, "$test");

    let mut client = Drawn::default();
    draw_frame(&mut hub, &mut client).await;
    assert_eq!(client.rgb, [200, 0, 0].repeat(SIZE.0 * SIZE.1));

    hub.send(&Instruction::new("key", ["65293", "1"])).await;
    hub.send(&Instruction::new("mouse", ["10", "999", "1"]))
        .await;
    hub.send(&Instruction::new("clipboard", ["4", "text/plain"]))
        .await;
    hub.send(&Instruction::new(
        "blob",
        ["4".to_owned(), base64(b"from the operator")],
    ))
    .await;
    hub.send(&Instruction::new("end", ["4"])).await;
    // The pointer is kept inside the 64 x 48 screen.
    assert_eq!(
        heard(&device, 3).await,
        ["key 65293 1", "pointer 10 47 1", "cut from the operator"]
    );

    let green = [0, 200, 0].repeat(SIZE.0 * SIZE.1);
    device
        .tell
        .send(Tell::Bytes(rfb::update(&[rfb::raw(ALL, &green)])))
        .await
        .expect("told");
    draw_frame(&mut hub, &mut client).await;
    assert_eq!(client.rgb, green);

    device
        .tell
        .send(Tell::Bytes(rfb::cut_text(b"from the device")))
        .await
        .expect("told");
    assert_eq!(hub.next_of("clipboard").await.args, ["1", "text/plain"]);
    assert_eq!(
        hub.next_of("blob").await.args[1],
        base64(b"from the device")
    );

    device.tell.send(Tell::Close).await.expect("told");
    hub.next_of("disconnect").await;
    assert_eq!(served.await.expect("served"), Ended::ByDevice);
}

#[tokio::test]
async fn the_clipboard_switches_hold_in_both_directions() {
    let device = Device::start(SignIn::None).await;
    let (mut hub, _served) = session(Settings::default());
    let off = [("disable-copy", "true"), ("disable-paste", "true")];
    hub.open("vnc", values(device.address, &off))
        .await
        .expect("ready");
    draw_frame(&mut hub, &mut Drawn::default()).await;

    hub.send(&Instruction::new("clipboard", ["4", "text/plain"]))
        .await;
    hub.send(&Instruction::new(
        "blob",
        ["4".to_owned(), base64(b"secret")],
    ))
    .await;
    hub.send(&Instruction::new("end", ["4"])).await;
    hub.send(&Instruction::new("key", ["97", "1"])).await;
    assert_eq!(heard(&device, 1).await, ["key 97 1"]);

    device
        .tell
        .send(Tell::Bytes(rfb::cut_text(b"from the device")))
        .await
        .expect("told");
    device
        .tell
        .send(Tell::Bytes(rfb::update(&[rfb::raw(
            ALL,
            &[9, 9, 9].repeat(SIZE.0 * SIZE.1),
        )])))
        .await
        .expect("told");
    // The next thing the hub gets is the frame, not the device's clipboard.
    let next = hub.next().await.expect("a frame");
    assert_ne!(next.opcode, "clipboard");
}

#[tokio::test]
async fn a_refused_sign_in_ends_the_session_as_unauthorized() {
    let device = Device::start(SignIn::Password { accepted: false }).await;
    let (mut hub, served) = session(Settings::default());
    hub.open("vnc", values(device.address, &[("password", "wrong")]))
        .await
        .expect("ready");
    assert_eq!(
        hub.next_of("error").await.args,
        ["Aborted. See logs.", "769"]
    );
    assert_eq!(served.await.expect("served"), Ended::SignInRefused);
}

#[tokio::test]
async fn a_device_that_is_not_there_ends_the_session_as_an_upstream_error() {
    let unused = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a port")
        .local_addr()
        .expect("an address");
    let (mut hub, served) = session(Settings::default());
    hub.open("vnc", values(unused, &[])).await.expect("ready");
    assert_eq!(hub.next_of("error").await.args[1], "519");
    assert_eq!(served.await.expect("served"), Ended::DeviceUnreachable);
}

#[tokio::test]
async fn a_missing_password_is_asked_for_and_asked_again_after_a_refused_answer() {
    let device = Device::start(SignIn::Password { accepted: true }).await;
    let (mut hub, _served) = session(Settings::default());
    hub.open("vnc", values(device.address, &[]))
        .await
        .expect("ready");
    assert_eq!(hub.next_of("required").await.args, ["password"]);

    // An empty answer is refused, and the gateway asks again.
    hub.send(&Instruction::new("argv", ["2", "text/plain", "password"]))
        .await;
    hub.send(&Instruction::new("end", ["2"])).await;
    assert_eq!(hub.next_of("required").await.args, ["password"]);

    hub.send(&Instruction::new("argv", ["3", "text/plain", "password"]))
        .await;
    hub.send(&Instruction::new(
        "blob",
        ["3".to_owned(), base64(b"secret12")],
    ))
    .await;
    hub.send(&Instruction::new("end", ["3"])).await;
    let mut client = Drawn::default();
    // The acknowledgements come first; the frame begins with `size`.
    client.receive(&[hub.next_of("size").await]);
    draw_frame(&mut hub, &mut client).await;
    assert_eq!(client.rgb, [200, 0, 0].repeat(SIZE.0 * SIZE.1));
}

#[tokio::test]
async fn bytes_a_device_may_not_send_end_the_session_as_an_upstream_error() {
    let device = Device::start(SignIn::None).await;
    let (mut hub, served) = session(Settings::default());
    hub.open("vnc", values(device.address, &[]))
        .await
        .expect("ready");
    draw_frame(&mut hub, &mut Drawn::default()).await;
    device.tell.send(Tell::Bytes(vec![99])).await.expect("told");
    assert_eq!(hub.next_of("error").await.args[1], "515");
    assert_eq!(served.await.expect("served"), Ended::DeviceFault);
}

#[tokio::test]
async fn a_handshake_that_does_not_finish_in_time_is_refused() {
    let settings = Settings {
        handshake_wait: Duration::from_millis(50),
        ..Settings::default()
    };
    let (mut hub, served) = session(settings);
    hub.send(&Instruction::new("select", ["vnc"])).await;
    assert_eq!(hub.next_of("error").await.args[1], "776");
    assert_eq!(served.await.expect("served"), Ended::Refused);
}

#[tokio::test]
async fn another_protocol_is_refused_or_passed_to_the_other_gateway() {
    let (mut hub, _served) = session(Settings::default());
    let refused = hub
        .open(
            "rdp",
            values("192.0.2.1:3389".parse().expect("an address"), &[]),
        )
        .await;
    assert_eq!(refused.expect_err("refused").args[1], "256");

    // The other gateway here answers every byte with `4.args,13.VERSION_1_5_0;` once.
    let other = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let address = other.local_addr().expect("an address").to_string();
    let seen = tokio::spawn(async move {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
        let (mut hub, _) = other.accept().await.expect("the gateway");
        let mut select = [0; 15];
        hub.read_exact(&mut select).await.expect("select");
        hub.write_all(b"4.args,13.VERSION_1_5_0;")
            .await
            .expect("args");
        select
    });
    let (mut hub, _served) = session(Settings {
        other_gateway: Some(address),
        ..Settings::default()
    });
    hub.send(&Instruction::new("select", ["rdp"])).await;
    assert_eq!(hub.next_of("args").await.args, ["VERSION_1_5_0"]);
    assert_eq!(&seen.await.expect("seen"), b"6.select,3.rdp;");
}
