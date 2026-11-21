//! The socket of the hub's own daemon in the hosted mode (HMD-071, HM-T35): only the hub's user
//! can open it, it reaches only the routes of K8, and each request has this computer as its peer.

use std::{fs, net::SocketAddr, os::unix::fs::PermissionsExt as _, path::PathBuf, time::Duration};

use axum::{Router, extract::ConnectInfo, routing::post};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
};

use super::{super::own_daemon_socket, nowhere};
use crate::fleet::access::listen::{Error, listen};

const DEADLINE: Duration = Duration::from_secs(10);

/// A route of K8 and a route beside it, each answering with the peer of the request.
fn router() -> Router {
    let peer = |ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.to_string() };
    Router::new()
        .route("/daemon/v1/push", post(peer))
        .route("/graphql", post(peer))
}

/// A hub that serves `router` on its loopback port and on the socket of its own daemon.
async fn hub(folder: &tempfile::TempDir) -> (PathBuf, tokio::task::JoinHandle<()>) {
    let socket = folder.path().join("own-daemon.sock");
    let listening = listen(
        "127.0.0.1:0".parse().unwrap(),
        None,
        std::sync::Arc::new(crate::clock::FixedClock(super::now())),
    )
    .await
    .unwrap();
    let own = own_daemon_socket(&socket).unwrap();
    let entrance = listening.admitting(nowhere()).with_own_daemon(Some(own));
    let serving = tokio::spawn(async move {
        entrance
            .serve_until(router(), std::future::pending(), Duration::ZERO)
            .await;
    });
    (socket, serving)
}

async fn answer(socket: &PathBuf, path: &str) -> String {
    let exchange = async {
        let mut stream = UnixStream::connect(socket).await.unwrap();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer).await;
        String::from_utf8_lossy(&answer).into_owned()
    };
    tokio::time::timeout(DEADLINE, exchange).await.unwrap()
}

#[tokio::test]
async fn hmd_071_the_own_daemon_reaches_only_the_routes_of_k8_as_this_computer() {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let (socket, serving) = hub(&folder).await;
    let mode = fs::symlink_metadata(&socket).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let pushed = answer(&socket, "/daemon/v1/push").await;
    assert!(pushed.starts_with("HTTP/1.1 200"), "{pushed}");
    assert!(pushed.ends_with("127.0.0.1:0"), "{pushed}");
    let beside = answer(&socket, "/graphql").await;
    assert!(beside.starts_with("HTTP/1.1 404"), "{beside}");
    serving.abort();
}

#[tokio::test]
async fn hmd_071_a_folder_other_users_can_reach_gets_no_socket_of_the_own_daemon() {
    let folder = tempfile::tempdir().unwrap();
    fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o750)).unwrap();
    let path = folder.path().join("own-daemon.sock");
    let refused = own_daemon_socket(&path).err();
    assert!(matches!(refused, Some(Error::OwnDaemonFolder(_))));
    assert!(!path.exists());
}
