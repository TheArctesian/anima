//! Stopping over TLS (R-040): on the stop signal the hub stops accepting, lets an open request
//! finish, and returns; a request that never finishes is dropped at the drain limit. Every
//! step has a deadline.

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Router, body::Bytes, routing::get};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    sync::oneshot,
    task::JoinHandle,
};
use tokio_rustls::{client::TlsStream, rustls::pki_types::ServerName};

use super::serving::connector;
use super::{Files, Throwaway, now, nowhere, throwaway};
use crate::clock::{Clock, FixedClock};

const DEADLINE: Duration = Duration::from_secs(20);

/// A TLS hub on a loopback port serving `GET /slow` (answered after 300 ms) and `POST /slow`
/// (answered once the body is read) until the sender
/// fires, then draining for at most `drain_limit`.
struct Hub {
    addr: SocketAddr,
    made: Throwaway,
    stop: oneshot::Sender<()>,
    serving: JoinHandle<()>,
    _files: Files,
}

async fn hub(drain_limit: Duration) -> Hub {
    let files = Files::new();
    let made = throwaway();
    files.write(&made.cert, &made.key, 0);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = super::super::listen(addr, Some(files.paths.clone()), clock)
        .await
        .unwrap();
    let addr = listening.local_addr().unwrap();
    let slow = get(|| async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        "done"
    });
    let router = Router::new().route("/slow", slow.post(|_: Bytes| async { "read" }));
    let (stop, signal) = oneshot::channel::<()>();
    let entrance = listening.admitting(nowhere());
    let serving = tokio::spawn(async move {
        let stop = async {
            signal.await.ok();
        };
        entrance.serve_until(router, stop, drain_limit).await;
    });
    Hub {
        addr,
        made,
        stop,
        serving,
        _files: files,
    }
}

async fn client(hub: &Hub, request: &[u8]) -> TlsStream<TcpStream> {
    let tcp = TcpStream::connect(hub.addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector(&hub.made).connect(name, tcp).await.unwrap();
    tls.write_all(request).await.unwrap();
    tls
}

impl Hub {
    /// Fires the stop signal; returns how long serving took to return.
    async fn stop(self) -> Duration {
        let began = Instant::now();
        self.stop.send(()).unwrap();
        tokio::time::timeout(DEADLINE, self.serving)
            .await
            .expect("serving returns after the stop signal")
            .unwrap();
        began.elapsed()
    }
}

#[tokio::test]
async fn r_040_stopping_over_tls_lets_an_open_request_finish_and_stops_accepting() {
    let hub = hub(Duration::from_secs(10)).await;
    let addr = hub.addr;
    let mut open = client(&hub, b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let answer = tokio::spawn(async move {
        let mut answer = String::new();
        let _ = open.read_to_string(&mut answer).await;
        answer
    });

    let took = hub.stop().await;

    let answer = tokio::time::timeout(DEADLINE, answer)
        .await
        .unwrap()
        .unwrap();
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.ends_with("done"), "{answer}");
    assert!(took < Duration::from_secs(5), "returned after {took:?}");
    assert!(
        TcpStream::connect(addr).await.is_err(),
        "no longer accepting"
    );
}

#[tokio::test]
async fn r_040_a_request_held_open_over_tls_is_dropped_at_the_drain_limit() {
    let limit = Duration::from_millis(300);
    let hub = hub(limit).await;
    let held = b"POST /slow HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1000\r\n\r\n{";
    let mut open = client(&hub, held).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let took = hub.stop().await;

    assert!(
        took >= limit,
        "returned after {took:?}, before the drain limit"
    );
    assert!(took < Duration::from_secs(5), "returned after {took:?}");
    let mut rest = Vec::new();
    let closed = tokio::time::timeout(DEADLINE, open.read_to_end(&mut rest)).await;
    assert!(closed.is_ok(), "the held connection is closed");
}
