//! RDP sessions that cannot open, and RDP that goes to the other gateway.

use anima_gateway::session::{Ended, Settings};
use guac_protocol::Instruction;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpListener;

use super::hub_side::{session, values};

const ACCOUNT: [(&str, &str); 3] = [
    ("username", "operator"),
    ("security", "tls"),
    ("ignore-cert", "true"),
];

#[tokio::test]
async fn a_device_that_is_not_an_rdp_server_ends_the_session_as_an_upstream_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("an address");
    tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.expect("the gateway");
        let _ = client.write_all(b"SSH-2.0-OpenSSH_9.9\r\n").await;
    });
    let (mut hub, served) = session(Settings::default());
    hub.open("rdp", values(address, &ACCOUNT))
        .await
        .expect("ready");

    assert_eq!(hub.next_of("error").await.args[1], "515");
    assert_eq!(served.await.expect("served"), Ended::NotRdp);
}

#[tokio::test]
async fn an_rdp_device_that_is_not_there_ends_the_session_as_not_found() {
    // A port that was free a moment ago: nothing listens on it.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("an address");
    drop(listener);
    let (mut hub, served) = session(Settings::default());
    hub.open("rdp", values(address, &ACCOUNT))
        .await
        .expect("ready");

    assert_eq!(hub.next_of("error").await.args[1], "519");
    assert_eq!(served.await.expect("served"), Ended::DeviceUnreachable);
}

#[tokio::test]
async fn rdp_goes_to_the_other_gateway_only_when_the_gateway_is_told_to_pass_it() {
    let other = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = other.local_addr().expect("an address").to_string();
    let seen = tokio::spawn(async move {
        use tokio::io::AsyncReadExt as _;
        let (mut hub, _) = other.accept().await.expect("the gateway");
        let mut select = [0; 15];
        hub.read_exact(&mut select).await.expect("select");
        hub.write_all(b"4.args,13.VERSION_1_5_0;")
            .await
            .expect("args");
        select
    });
    // Not told: the gateway answers `select rdp` itself, with its own names.
    let (mut hub, _served) = session(Settings {
        other_gateway: Some(address.clone()),
        ..Settings::default()
    });
    hub.send(&Instruction::new("select", ["rdp"])).await;
    assert!(
        hub.next_of("args")
            .await
            .args
            .contains(&"cert-fingerprints".to_owned())
    );

    let (mut hub, _served) = session(Settings {
        other_gateway: Some(address),
        passed: vec!["rdp".to_owned()],
        ..Settings::default()
    });
    hub.send(&Instruction::new("select", ["rdp"])).await;
    assert_eq!(hub.next_of("args").await.args, ["VERSION_1_5_0"]);
    assert_eq!(&seen.await.expect("seen"), b"6.select,3.rdp;");
}
