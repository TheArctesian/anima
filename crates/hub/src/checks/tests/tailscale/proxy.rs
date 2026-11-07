//! A fake `LocalAPI` proxy: records each request head and body length, answers as told.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[derive(Clone, Debug)]
pub enum Reply {
    Answer(u16, String),
    /// Accepts the request and never answers.
    Hang,
}

impl Reply {
    pub fn json(status: u16, body: &str) -> Self {
        Self::Answer(status, body.to_owned())
    }
}

/// A request as the proxy saw it.
#[derive(Debug)]
pub struct Seen {
    pub head: String,
    /// Bytes received after the head, within a short wait.
    pub body_len: usize,
}

pub struct Proxy {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
    pub connections: Arc<AtomicUsize>,
}

impl Proxy {
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub fn seen(&self) -> Vec<Seen> {
        std::mem::take(&mut self.seen.lock().unwrap())
    }
}

async fn serve_one(mut stream: tokio::net::TcpStream, reply: Reply, seen: Arc<Mutex<Vec<Seen>>>) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read_exact(&mut byte).await.is_err() {
            return;
        }
        head.push(byte[0]);
    }
    let mut rest = [0u8; 64];
    let body_len = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        stream.read(&mut rest),
    )
    .await
    .map_or(0, Result::unwrap_or_default);
    seen.lock().unwrap().push(Seen {
        head: String::from_utf8_lossy(&head).into_owned(),
        body_len,
    });
    match reply {
        Reply::Answer(status, body) => {
            let text = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(text.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
        Reply::Hang => tokio::time::sleep(std::time::Duration::from_secs(30)).await,
    }
}

pub async fn start(answer: impl Fn() -> Reply + Send + Sync + 'static) -> Proxy {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = Proxy {
        port: listener.local_addr().unwrap().port(),
        seen: Arc::default(),
        connections: Arc::default(),
    };
    let (seen, connections) = (proxy.seen.clone(), proxy.connections.clone());
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            connections.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_one(stream, answer(), seen.clone()));
        }
    });
    proxy
}
