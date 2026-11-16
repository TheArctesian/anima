//! What a device or a listener on the way may not get from the gateway: the account's
//! name before TLS, and room in the worker's memory for channel data without end.

use anima_gateway::handshake::{Certificate, Security};
use anima_gateway::rdp::{Device as Session, Malformed};
use anima_gateway::screen::Screen;
use ironrdp_cliprdr::CliprdrClient;
use ironrdp_core::encode_vec;
use ironrdp_pdu::Action;
use ironrdp_pdu::mcs::{McsMessage, SendDataIndication};
use ironrdp_pdu::x224::X224;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpListener;

use super::device::Device;
use super::opening::{TLS, USERNAME, opened, wanted};

const FIRST: u32 = 0x1;
const PACKED: u32 = 0x0020_0000;

#[tokio::test]
async fn the_first_message_which_is_not_encrypted_does_not_name_the_account() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("an address");
    let heard = tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.expect("the gateway");
        let mut first = vec![0; 512];
        let count = client.read(&mut first).await.expect("the request");
        first.truncate(count);
        first
    });

    let _ = opened(
        address,
        wanted(Security::Nla, Certificate::Accepted([0xee; 32])),
    )
    .await;

    let first = heard.await.expect("heard");
    let names_the_account = first
        .windows(USERNAME.len())
        .any(|part| part == USERNAME.as_bytes());
    assert!(!first.is_empty() && !names_the_account);
}

/// A part of a clipboard channel message from the device.
fn part(channel: u16, whole: u32, flags: u32, bytes: usize) -> Vec<u8> {
    let mut data = whole.to_le_bytes().to_vec();
    data.extend_from_slice(&flags.to_le_bytes());
    data.resize(8 + bytes, 0x41);
    let message = McsMessage::SendDataIndication(SendDataIndication {
        initiator_id: 1002,
        channel_id: channel,
        user_data: data.into(),
    });
    encode_vec(&X224(message)).expect("encodes")
}

/// A session on the test device, its screen, and the number of its clipboard channel.
async fn signed_in() -> (Device, Session, Screen, u16) {
    let device = Device::start(TLS);
    let opened = opened(device.address, wanted(Security::Tls, Certificate::Any))
        .await
        .expect("opened");
    let channels = &opened.result.static_channels;
    let clipboard = channels
        .get_channel_id_by_type::<CliprdrClient>()
        .expect("joined");
    let screen = Screen::new(320, 240).expect("a screen");
    (device, Session::new(opened.result), screen, clipboard)
}

#[tokio::test]
async fn packed_channel_data_ends_the_session_because_no_packing_was_agreed() {
    let (_device, mut session, mut screen, clipboard) = signed_in().await;
    let message = part(clipboard, 16, FIRST | PACKED, 1000);

    assert_eq!(
        session.receive(Action::X224, &message, &mut screen),
        Err(Malformed)
    );
}

#[tokio::test]
async fn a_channel_message_longer_than_a_clipboard_text_may_be_ends_the_session() {
    let (_device, mut session, mut screen, clipboard) = signed_in().await;
    // Declared too long.
    let declared = part(clipboard, 64 << 20, FIRST, 1000);
    assert_eq!(
        session.receive(Action::X224, &declared, &mut screen),
        Err(Malformed)
    );

    // Declared short, and then it does not end.
    let (_device, mut session, mut screen, clipboard) = signed_in().await;
    let first = part(clipboard, 1 << 20, FIRST, 60_000);
    assert!(session.receive(Action::X224, &first, &mut screen).is_ok());
    let more = part(clipboard, 1 << 20, 0, 60_000);
    let ended = (0..40).any(|_| session.receive(Action::X224, &more, &mut screen).is_err());
    assert!(ended);
}
