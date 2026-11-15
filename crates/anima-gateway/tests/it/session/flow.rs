//! A session keeps to the pace of both sides: frames wait for the client, update requests
//! wait for the device, and a side that stops reading cannot hold the session.

use std::time::Duration;

use anima_gateway::listening::{Workers, listen};
use anima_gateway::session::{Ended, Settings};
use base64::Engine as _;
use guac_protocol::Instruction;
use tokio::net::{TcpListener, TcpStream};

use super::device::{ALL, ASKED_FOR_ALL, ASKED_FOR_CHANGES, Device, SIZE, SignIn, Tell};
use super::hub_side::{HubSide, session, values};
use crate::drawn::Drawn;
use crate::vnc::rfb;

/// Tells the device to show one colour on all of its screen.
async fn show(device: &Device, colour: [u8; 3]) {
    let update = rfb::update(&[rfb::raw(ALL, &colour.repeat(SIZE.0 * SIZE.1))]);
    device.tell.send(Tell::Bytes(update)).await.expect("told");
}

#[tokio::test]
async fn frames_wait_for_the_client_and_requests_wait_for_the_device() {
    let device = Device::start(SignIn::None).await;
    let (mut hub, _served) = session(Settings::default());
    hub.open("vnc", values(device.address, &[]))
        .await
        .expect("ready");

    // Two frames go out without an answer; the third waits for one.
    let first = hub.next_of("sync").await;
    show(&device, [0, 200, 0]).await;
    hub.next_of("sync").await;
    show(&device, [0, 0, 200]).await;
    assert!(hub.is_quiet_for(Duration::from_millis(400)).await);
    hub.send(&first).await;
    let mut client = Drawn::black(SIZE.0, SIZE.1);
    loop {
        let instruction = hub.next().await.expect("the third frame");
        let sync = instruction.opcode == "sync";
        client.receive(&[instruction]);
        if sync {
            break;
        }
    }
    assert_eq!(client.rgb, [0, 0, 200].repeat(SIZE.0 * SIZE.1));

    // One request for all of the screen, then one request after each update: none while
    // the device has not answered the last.
    assert_eq!(
        device.asked(),
        [
            ASKED_FOR_ALL,
            ASKED_FOR_CHANGES,
            ASKED_FOR_CHANGES,
            ASKED_FOR_CHANGES
        ]
    );
}

#[tokio::test]
async fn a_device_that_stops_reading_cannot_hold_the_session() {
    let device = Device::start(SignIn::None).await;
    let settings = Settings {
        write_wait: Duration::from_millis(300),
        ..Settings::default()
    };
    let (mut hub, served) = session(settings);
    hub.open("vnc", values(device.address, &[]))
        .await
        .expect("ready");
    hub.next_of("sync").await;
    device.tell.send(Tell::StopReading).await.expect("told");

    // Far more clipboard text than the connection to the device can hold.
    let blob = base64::engine::general_purpose::STANDARD.encode([b'x'; 6000]);
    'pasting: for stream in 0..400 {
        let stream = stream.to_string();
        let clipboard = Instruction::new("clipboard", [stream.as_str(), "text/plain"]);
        let blob = Instruction::new("blob", [stream.clone(), blob.clone()]);
        let parts = std::iter::once(&clipboard).chain(std::iter::repeat_n(&blob, 40));
        for part in parts {
            // The session ending is what the test waits for.
            if served.is_finished() || !hub.try_send(part).await {
                break 'pasting;
            }
        }
        hub.try_send(&Instruction::new("end", [stream])).await;
    }
    let ended = tokio::time::timeout(Duration::from_secs(10), served).await;
    assert_eq!(
        ended.expect("the session ends").expect("served"),
        Ended::ByDevice
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_gateway_says_it_is_busy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("an address");
    let workers = Workers {
        program: env!("CARGO_BIN_EXE_anima-gateway").into(),
        other_gateway: None,
        most_sessions: 1,
    };
    let listening = tokio::spawn(listen(listener, workers));

    let device = Device::start(SignIn::None).await;
    let mut first = HubSide::on(TcpStream::connect(address).await.expect("connected"));
    first
        .open("vnc", values(device.address, &[]))
        .await
        .expect("ready");
    let mut second = HubSide::on(TcpStream::connect(address).await.expect("connected"));
    assert_eq!(
        second.next_of("error").await.args,
        ["Aborted. See logs.", "513"]
    );
    assert!(second.next().await.is_none());
    listening.abort();
}
