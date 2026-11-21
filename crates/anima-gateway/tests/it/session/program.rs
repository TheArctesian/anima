//! The real program: the listener, and one worker process for each session.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt as _, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

use super::device::{Device, SIZE, SignIn, Tell};
use super::draw_frame;
use super::hub_side::{HubSide, values};
use crate::drawn::Drawn;

/// Starts the program on a port that it chooses, and returns it with the port from its first line.
/// A port chosen here and freed first could be taken by a different test before the program binds
/// it; the test then spoke to that listener (seen under load: `InvalidLength` at byte 0).
async fn start_program() -> (Child, u16) {
    let mut program = Command::new(crate::program())
        .args(["listen", "127.0.0.1:0"])
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("the program starts");
    let mut lines = BufReader::new(program.stderr.take().expect("its standard error")).lines();
    let first = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("the program listens in time")
        .expect("its standard error is read")
        .expect("the program says where it listens");
    let port = first
        .rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("a port in {first:?}"));
    // The rest is read and dropped, so that the program never waits on a full pipe.
    tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
    (program, port)
}

async fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the program accepts")
}

#[tokio::test(flavor = "multi_thread")]
async fn each_session_has_its_own_worker_and_a_failed_one_leaves_the_others() {
    let (mut program, port) = start_program().await;

    let (first_device, second_device) = (
        Device::start(SignIn::None).await,
        Device::start(SignIn::None).await,
    );
    let mut first = HubSide::on(connect(port).await);
    let mut second = HubSide::on(connect(port).await);
    let first_id = first
        .open("vnc", values(first_device.address, &[]))
        .await
        .expect("ready");
    let second_id = second
        .open("vnc", values(second_device.address, &[]))
        .await
        .expect("ready");
    assert_ne!(first_id, second_id);
    let (mut first_client, mut second_client) = (Drawn::default(), Drawn::default());
    draw_frame(&mut first, &mut first_client).await;
    draw_frame(&mut second, &mut second_client).await;
    assert_eq!(first_client.rgb, [200, 0, 0].repeat(SIZE.0 * SIZE.1));

    // The first session's device breaks the protocol: that session ends.
    first_device
        .tell
        .send(Tell::Bytes(vec![99]))
        .await
        .expect("told");
    assert_eq!(first.next_of("error").await.args[1], "515");
    assert!(first.next().await.is_none());

    // The second goes on, and a third can open.
    let blue = [0, 0, 200].repeat(SIZE.0 * SIZE.1);
    let update = crate::vnc::rfb::update(&[crate::vnc::rfb::raw(super::ALL, &blue)]);
    second_device
        .tell
        .send(Tell::Bytes(update))
        .await
        .expect("told");
    draw_frame(&mut second, &mut second_client).await;
    assert_eq!(second_client.rgb, blue);
    let third_device = Device::start(SignIn::None).await;
    let mut third = HubSide::on(connect(port).await);
    third
        .open("vnc", values(third_device.address, &[]))
        .await
        .expect("ready");
    drop((second, third));
    program
        .kill()
        .await
        .expect("the listener stops and is reaped");
}
