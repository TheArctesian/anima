//! A fake `LocalAPI` proxy for GET requests: records each request head, answers as told.
//!
//! A copy of `crates/hub/src/vpn/tests/proxy.rs`: `tests/it` is another crate and cannot import
//! the hub's `#[cfg(test)]` code.

use std::sync::{Arc, Mutex};

use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
};

/// The status and body the proxy answers with.
#[derive(Clone, Debug)]
pub struct Reply {
    status: u16,
    body: String,
}

impl Reply {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
        }
    }
}

pub struct Proxy {
    port: u16,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Proxy {
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// The request heads seen so far.
    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

async fn serve_one(mut stream: TcpStream, reply: Reply, seen: Arc<Mutex<Vec<String>>>) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read_exact(&mut byte).await.is_err() {
            return;
        }
        head.push(byte[0]);
    }
    seen.lock()
        .unwrap()
        .push(String::from_utf8_lossy(&head).into_owned());
    let Reply { status, body } = reply;
    let text = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(text.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// A proxy that answers each request with `answer()`; the closure may count calls.
pub async fn start(answer: impl Fn() -> Reply + Send + Sync + 'static) -> Proxy {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = Proxy {
        port: listener.local_addr().unwrap().port(),
        seen: Arc::default(),
    };
    let seen = proxy.seen.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_one(stream, answer(), seen.clone()));
        }
    });
    proxy
}
