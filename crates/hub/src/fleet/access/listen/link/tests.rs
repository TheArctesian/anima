//! The link's socket: who can open it, which header names a device, and that a connection
//! through it is served with that device as its peer (NET-002 and NET-003 in the link mode,
//! LNK-002). The header bytes are the file the link's own test writes against.

use std::{
    fs,
    net::SocketAddr,
    os::unix::fs::{FileTypeExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{Router, extract::ConnectInfo, routing::get};
use chrono::{TimeZone as _, Utc};
use ipnet::IpNet;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream, UnixStream},
    task::JoinHandle,
};

use super::{
    super::{
        Error, listen,
        serve::{self, Limits, SERVED, Serving, Ways},
    },
    header, link_socket,
};
use crate::{
    clock::{Clock, FixedClock},
    config::LINK_DEVICES,
    fleet::access::layer::Edge,
};

/// What `link/hublink` sends for the device `fd41:4e49:4d41:1:2:3:4:5` on its port 40000.
const FROM_THE_LINK: &[u8; header::LENGTH] =
    include_bytes!("../../../../../../../link/hublink/testdata/device-header.bin");

const DEADLINE: Duration = Duration::from_secs(10);

fn clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(
        Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap(),
    ))
}

/// An edge whose permitted networks are those of the link mode's devices.
fn link_edge() -> Edge {
    Edge::permitting(&[IpNet::V6(LINK_DEVICES)], clock())
}

/// A header as the link writes it, with `change` applied to its bytes.
fn header_with(change: impl FnOnce(&mut [u8; header::LENGTH])) -> [u8; header::LENGTH] {
    let mut bytes = *FROM_THE_LINK;
    change(&mut bytes);
    bytes
}

fn source(address: &str) -> [u8; header::LENGTH] {
    let address: std::net::Ipv6Addr = address.parse().unwrap();
    header_with(|bytes| bytes[16..32].copy_from_slice(&address.octets()))
}

#[test]
fn the_links_header_names_the_device_by_its_link_address_and_port() {
    let device = header::device(FROM_THE_LINK).unwrap();
    let want: SocketAddr = "[fd41:4e49:4d41:1:2:3:4:5]:40000".parse().unwrap();
    assert_eq!(device, want);
}

#[test]
fn net_002_a_header_names_no_address_outside_the_link_range() {
    let outside = [
        "::1",
        "::ffff:127.0.0.1",
        "::ffff:100.64.0.1",
        "fd41:4e49:4d40::1",
        "fd41:4e49:4d42::1",
        "fd7a:115c:a1e0::1",
        "2001:db8::1",
    ];
    for address in outside {
        assert_eq!(header::device(&source(address)), None, "{address}");
    }
    let inside = "fd41:4e49:4d41:ffff:ffff:ffff:ffff:ffff";
    assert!(header::device(&source(inside)).is_some());
}

#[test]
fn net_002_bytes_that_are_not_the_links_header_name_no_device() {
    // The 12 fixed bytes, the version and command, the family and the length.
    for at in 0..16 {
        let changed = header_with(|bytes| bytes[at] ^= 0x01);
        assert_eq!(header::device(&changed), None, "byte {at}");
    }
    let request = header_with(|bytes| {
        let text = b"GET / HTTP/1.1\r\nHost: hub\r\nConnection: close\r\n\r\n....";
        bytes.copy_from_slice(&text[..header::LENGTH]);
    });
    assert_eq!(header::device(&request), None);
}

/// A folder only this user can reach, and the path of a socket in it.
fn private_folder() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = folder.path().join("link.sock");
    (folder, path)
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777
}

#[tokio::test]
async fn net_003_the_socket_is_made_for_the_hubs_user_only_and_replaces_an_old_one() {
    let (_folder, path) = private_folder();
    let first = link_socket(&path).unwrap();
    assert!(fs::symlink_metadata(&path).unwrap().file_type().is_socket());
    assert_eq!(mode(&path), 0o600);
    // A hub that stopped leaves its socket; the next start takes the path again.
    drop(first);
    link_socket(&path).unwrap();
    assert_eq!(mode(&path), 0o600);
}

#[tokio::test]
async fn net_003_a_folder_other_users_can_reach_gets_no_socket() {
    for shared in [0o750, 0o705, 0o701, 0o777] {
        let (folder, path) = private_folder();
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(shared)).unwrap();
        let refused = link_socket(&path).err();
        assert!(
            matches!(refused, Some(Error::LinkFolder(_))),
            "mode {shared:o}"
        );
        assert!(!path.exists(), "mode {shared:o}: a socket was left");
    }
    let missing = Path::new("/nonexistent-anima-folder/link.sock");
    let refused = link_socket(missing).err();
    assert!(matches!(refused, Some(Error::LinkFolder(_))));
}

#[tokio::test]
async fn net_003_a_folder_reached_through_a_symbolic_link_gets_no_socket() {
    let (folder, _) = private_folder();
    let real = folder.path().join("real");
    fs::create_dir(&real).unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
    let alias = folder.path().join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let refused = link_socket(&alias.join("link.sock")).err();
    assert!(matches!(refused, Some(Error::LinkFolder(_))));
    assert!(!real.join("link.sock").exists());
}

#[tokio::test]
async fn a_file_that_is_not_a_socket_is_refused_and_left_as_it_is() {
    let (_folder, path) = private_folder();
    fs::write(&path, "the operator's file").unwrap();
    let refused = link_socket(&path).err();
    assert!(matches!(refused, Some(Error::LinkPath(_))));
    assert_eq!(fs::read_to_string(&path).unwrap(), "the operator's file");
}

/// A hub in the link mode: its listener on loopback and its socket, serving a router that
/// answers `GET /` with the peer of the connection. Stops when dropped.
struct LinkedHub {
    _folder: tempfile::TempDir,
    socket: PathBuf,
    edge: Edge,
    serving: JoinHandle<()>,
}

impl Drop for LinkedHub {
    fn drop(&mut self) {
        self.serving.abort();
    }
}

fn peer_router() -> Router {
    Router::new().route(
        "/",
        get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.to_string() }),
    )
}

async fn linked_hub(edge: Edge) -> LinkedHub {
    let (folder, socket) = private_folder();
    let listening = listen("127.0.0.1:0".parse().unwrap(), None, clock())
        .await
        .unwrap();
    let link = link_socket(&socket).unwrap();
    let router = peer_router();
    let entrance = listening.admitting(edge.clone()).with_link(Some(link));
    let serving = tokio::spawn(async move {
        let stop = std::future::pending();
        entrance.serve_until(router, stop, Duration::ZERO).await;
    });
    LinkedHub {
        _folder: folder,
        socket,
        edge,
        serving,
    }
}

const GET: &[u8] = b"GET / HTTP/1.1\r\nHost: hub\r\nConnection: close\r\n\r\n";

/// Everything the hub answers on its socket to `sent`, until it closes the connection.
async fn answer_to(hub: &LinkedHub, sent: &[&[u8]]) -> String {
    let exchange = async {
        let mut stream = UnixStream::connect(&hub.socket).await.unwrap();
        for part in sent {
            // The hub may close before everything is written.
            let _ = stream.write_all(part).await;
        }
        let _ = stream.shutdown().await;
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer).await;
        String::from_utf8_lossy(&answer).into_owned()
    };
    tokio::time::timeout(DEADLINE, exchange)
        .await
        .expect("the hub must answer or close")
}

#[tokio::test]
async fn lnk_002_a_request_through_the_socket_has_the_named_device_as_its_peer() {
    let hub = linked_hub(link_edge()).await;
    let answer = answer_to(&hub, &[FROM_THE_LINK, GET]).await;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(
        answer.ends_with("[fd41:4e49:4d41:1:2:3:4:5]:40000"),
        "{answer}"
    );
    let other = source("fd41:4e49:4d41:9::9");
    let answer = answer_to(&hub, &[&other, GET]).await;
    assert!(answer.ends_with("[fd41:4e49:4d41:9::9]:40000"), "{answer}");
}

#[tokio::test]
async fn net_002_a_connection_that_names_no_device_is_closed_with_no_answer() {
    let hub = linked_hub(link_edge()).await;
    let loopback = source("::ffff:127.0.0.1");
    let unnamed: [&[&[u8]]; 4] = [&[GET], &[&loopback, GET], &[&FROM_THE_LINK[..20]], &[]];
    for sent in unnamed {
        assert_eq!(answer_to(&hub, sent).await, "", "{sent:?}");
    }
    // The socket still serves the link after them.
    let answer = answer_to(&hub, &[FROM_THE_LINK, GET]).await;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
}

#[tokio::test]
async fn net_002_a_connection_that_stays_silent_is_closed_at_the_deadline() {
    let hub = linked_hub(link_edge()).await;
    let mut silent = UnixStream::connect(&hub.socket).await.unwrap();
    let mut answer = Vec::new();
    let closed = tokio::time::timeout(DEADLINE, silent.read_to_end(&mut answer)).await;
    assert!(closed.is_ok(), "the hub kept a silent connection open");
    assert_eq!(answer, b"");
}

#[tokio::test]
async fn net_002_a_device_outside_the_permitted_networks_is_closed_with_no_answer() {
    // The edge is asked for a link connection as for any other: here it permits no network.
    let hub = linked_hub(Edge::permitting(&[], clock())).await;
    assert_eq!(answer_to(&hub, &[FROM_THE_LINK, GET]).await, "");
}

#[tokio::test]
async fn net_002_a_connection_that_names_no_device_is_written_once_a_minute() {
    let hub = linked_hub(link_edge()).await;
    let first = hub.edge.unnamed();
    assert_eq!(
        first.messages(),
        [
            "Refused a connection on the link's socket: it did not start with the header \
             that names a device of the link."
        ]
    );
    assert_eq!(hub.edge.unnamed().messages(), [] as [String; 0]);
}

#[tokio::test]
async fn clients_of_the_loopback_port_cannot_take_the_places_of_the_links_devices() {
    let (folder, socket) = private_folder();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let ways = Ways {
        tcp,
        link: Some(link_socket(&socket).unwrap()),
        own_daemon: None,
    };
    let limits = Limits {
        connections: 2,
        connections_per_peer: 2,
        ..SERVED
    };
    // Loopback is permitted, as in the link mode, so its idle connections hold places.
    let both = [IpNet::V6(LINK_DEVICES), "127.0.0.0/8".parse().unwrap()];
    let edge = Edge::permitting(&both, clock());
    let serving = Serving::new(peer_router(), edge.clone(), None, limits);
    let run = serve::run(ways, serving, std::future::pending(), Duration::ZERO);
    let hub = LinkedHub {
        _folder: folder,
        socket,
        edge,
        serving: tokio::spawn(run),
    };

    let _first = TcpStream::connect(addr).await.unwrap();
    let _second = TcpStream::connect(addr).await.unwrap();
    let mut third = TcpStream::connect(addr).await.unwrap();
    let mut none = Vec::new();
    let full = tokio::time::timeout(DEADLINE, third.read_to_end(&mut none)).await;
    assert!(
        full.is_ok(),
        "the listener's places must be taken for this test"
    );

    let answer = answer_to(&hub, &[FROM_THE_LINK, GET]).await;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
}
