//! The connect function on a link address (HMD-048), against a fake dial socket of the link:
//! the request line, the connection after `ok`, and the error of each other word. The fake
//! also serves the tests of the checks and of SSH that go through the link.

use std::{fs, io::ErrorKind, net::SocketAddr, os::unix::fs::PermissionsExt as _};

use tokio::{
    io::{
        AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader, copy_bidirectional,
    },
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
};

use crate::fleet::targets::{AddressPolicy, Reached, connect, on_link};

/// The link address of the fake link's one computer.
pub(crate) const COMPUTER: &str = "fd41:4e49:4d41::7";

/// What the fake link does with a request.
#[derive(Clone, Copy)]
pub(crate) enum Link {
    /// The computer is this one: the link connects to the port on loopback, as the link
    /// program of a computer does, and says `closed` when nothing listens.
    Carries,
    /// The link says this word and ends the connection.
    Says(&'static str),
}

/// A policy whose link is a fake dial socket, and the folder that holds the socket.
pub(crate) fn link(behaviour: Link) -> (tempfile::TempDir, AddressPolicy) {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = folder.path().join("dial.sock");
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(stream, behaviour));
        }
    });
    (folder, AddressPolicy::default().reaching_link(Some(path)))
}

async fn serve(stream: UnixStream, behaviour: Link) {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    stream.read_line(&mut line).await.unwrap();
    let port = line
        .strip_prefix(&format!("dial {COMPUTER} "))
        .and_then(|rest| rest.strip_suffix('\n'))
        .and_then(|port| port.parse::<u16>().ok());
    let (Link::Carries, Some(port)) = (behaviour, port) else {
        let word = match behaviour {
            Link::Says(word) => word,
            Link::Carries => "bad_request",
        };
        let _ = stream.write_all(format!("{word}\n").as_bytes()).await;
        return;
    };
    let Ok(mut local) = TcpStream::connect(("127.0.0.1", port)).await else {
        let _ = stream.write_all(b"closed\n").await;
        return;
    };
    stream.write_all(b"ok\n").await.unwrap();
    let _ended = copy_bidirectional(&mut stream, &mut local).await;
}

fn computer(port: u16) -> SocketAddr {
    SocketAddr::new(COMPUTER.parse().unwrap(), port)
}

#[tokio::test]
async fn hmd_048_a_link_address_is_reached_through_the_link() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        // The first bytes of the device come right after the link's line.
        stream.write_all(b"SSH-2.0-device\r\n").await.unwrap();
        let mut heard = [0; 4];
        stream.read_exact(&mut heard).await.unwrap();
        stream.write_all(&heard).await.unwrap();
    });
    let (_folder, policy) = link(Link::Carries);

    let mut reached = connect(&policy, computer(port)).await.unwrap();

    assert!(matches!(reached, Reached::Link(_)));
    assert_eq!(reached.network_peer(), None);
    let mut banner = [0; 16];
    reached.read_exact(&mut banner).await.unwrap();
    assert_eq!(&banner, b"SSH-2.0-device\r\n");
    reached.write_all(b"ping").await.unwrap();
    let mut echoed = [0; 4];
    reached.read_exact(&mut echoed).await.unwrap();
    assert_eq!(&echoed, b"ping");
}

#[tokio::test]
async fn hmd_048_each_word_of_the_link_is_an_error_of_its_kind() {
    for (word, kind) in [
        ("refused", ErrorKind::PermissionDenied),
        ("closed", ErrorKind::ConnectionRefused),
        ("not_connected", ErrorKind::NotConnected),
        ("timeout", ErrorKind::TimedOut),
        ("bad_request", ErrorKind::InvalidData),
        ("okay", ErrorKind::InvalidData),
        (
            "a word that is longer than any word of the link",
            ErrorKind::InvalidData,
        ),
    ] {
        let (_folder, policy) = link(Link::Says(word));
        let error = connect(&policy, computer(22)).await.unwrap_err();
        assert_eq!(error.kind(), kind, "{word}");
    }
}

#[tokio::test]
async fn hmd_048_a_hub_whose_link_does_not_run_reaches_no_computer_of_it() {
    let folder = tempfile::tempdir().unwrap();
    let policy = AddressPolicy::default().reaching_link(Some(folder.path().join("dial.sock")));
    let error = connect(&policy, computer(22)).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn an_address_of_the_network_is_reached_by_the_network_also_with_a_link() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (_folder, policy) = link(Link::Says("refused"));

    let reached = connect(&policy, address).await.unwrap();

    assert!(matches!(reached, Reached::Tcp(_)));
    assert_eq!(reached.network_peer(), Some(address.ip()));
}

#[test]
fn only_the_range_of_the_link_is_on_the_link() {
    assert!(on_link(COMPUTER.parse().unwrap()));
    assert!(on_link("fd41:4e49:4d41:ffff::1".parse().unwrap()));
    for other in [
        "fd41:4e49:4d42::7",
        "fd00::7",
        "100.64.0.7",
        "::1",
        "10.0.0.7",
    ] {
        assert!(!on_link(other.parse().unwrap()), "{other}");
    }
}
