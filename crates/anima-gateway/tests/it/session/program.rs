//! The real program: the listener, and one worker process for each session.

use std::process::Stdio;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::process::Command;

use super::device::{Device, SIZE, SignIn, Tell};
use super::draw_frame;
use super::hub_side::{HubSide, values};
use crate::drawn::Drawn;

/// Connects to the program's port once it listens.
async fn connect(port: u16) -> TcpStream {
    for _ in 0..200 {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)).await {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the program does not listen");
}

#[tokio::test(flavor = "multi_thread")]
async fn each_session_has_its_own_worker_and_a_failed_one_leaves_the_others() {
    let free = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = free.local_addr().expect("an address").port();
    drop(free);
    let mut program = Command::new(crate::program())
        .args(["listen", &format!("127.0.0.1:{port}")])
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("the program starts");

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
