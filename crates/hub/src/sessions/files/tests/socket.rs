//! The operator's client as the app opens it: a WebSocket to `/files?token=…` with the
//! subprotocol `anima-files`, JSON text for requests and binary frames for upload bytes
//! (K4 Section 9).

use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::Value;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        Error, Message,
        client::IntoClientRequest as _,
        http::StatusCode,
        protocol::{
            CloseFrame,
            frame::{
                Frame,
                coding::{CloseCode, Data, OpCode},
            },
        },
    },
};

use super::WAIT;

/// What the client got next, the hub's pings skipped.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Received {
    Text(Value),
    Binary(Vec<u8>),
    Closed(Option<u16>),
}

/// An open files tunnel from the client's side.
pub(super) struct Socket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    /// Every hub ping seen so far.
    pub(super) pings: usize,
}

/// Opens the files tunnel at `url`, offering `anima-files` when `subprotocol` is set. A
/// refusal before the upgrade gives its HTTP status.
pub(super) async fn open_with(url: &str, subprotocol: bool) -> Result<Socket, StatusCode> {
    let mut request = url.into_client_request().unwrap();
    if subprotocol {
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "anima-files".parse().unwrap());
    }
    match timeout(WAIT, connect_async(request)).await.unwrap() {
        Ok((stream, response)) => {
            assert_eq!(response.headers()["Sec-WebSocket-Protocol"], "anima-files");
            Ok(Socket { stream, pings: 0 })
        }
        Err(Error::Http(response)) => {
            assert!(
                response.body().as_ref().is_none_or(Vec::is_empty),
                "a refusal has no body detail"
            );
            Err(response.status())
        }
        Err(other) => panic!("the files request failed: {other}"),
    }
}

/// Opens the files tunnel as the app does.
pub(super) async fn open(url: &str) -> Socket {
    open_with(url, true).await.expect("the files tunnel opens")
}

impl Socket {
    /// The next message or the close, skipping WebSocket pings and the hub's `ping`.
    pub(super) async fn next(&mut self) -> Received {
        loop {
            let message = timeout(WAIT, self.stream.next()).await.expect("a message");
            match message {
                Some(Ok(Message::Text(text))) => {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value == serde_json::json!({ "type": "ping" }) {
                        self.pings += 1;
                        continue;
                    }
                    return Received::Text(value);
                }
                Some(Ok(Message::Binary(bytes))) => return Received::Binary(bytes.to_vec()),
                Some(Ok(Message::Close(frame))) => {
                    return Received::Closed(frame.map(|frame| u16::from(frame.code)));
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Err(_)) | None => return Received::Closed(None),
            }
        }
    }

    /// The next text message, which must come before anything else.
    pub(super) async fn text(&mut self) -> Value {
        match self.next().await {
            Received::Text(value) => value,
            other => panic!("{other:?} while a text message was due"),
        }
    }

    /// The close code, after any messages still on the way.
    pub(super) async fn close_code(&mut self) -> Option<u16> {
        loop {
            if let Received::Closed(code) = self.next().await {
                return code;
            }
        }
    }

    /// The `download` size, then the binary frames up to it, then `done`.
    pub(super) async fn download(&mut self, path: &str) -> Vec<u8> {
        self.send(&format!(r#"{{"type":"download","path":"{path}"}}"#))
            .await;
        let announced = self.text().await;
        assert_eq!(announced["type"], "download", "{announced}");
        let size = usize::try_from(announced["size"].as_u64().unwrap()).unwrap();
        let mut bytes = Vec::new();
        while bytes.len() < size {
            match self.next().await {
                Received::Binary(more) => {
                    assert!(!more.is_empty() && more.len() <= 64 * 1024);
                    bytes.extend(more);
                }
                other => panic!("{other:?} while download bytes were due"),
            }
        }
        assert_eq!(bytes.len(), size, "exactly the announced size");
        assert_eq!(self.text().await, serde_json::json!({ "type": "done" }));
        bytes
    }

    pub(super) async fn send(&mut self, text: &str) {
        self.stream.send(Message::text(text)).await.unwrap();
    }

    pub(super) async fn send_binary(&mut self, bytes: &[u8]) {
        self.stream
            .send(Message::binary(bytes.to_vec()))
            .await
            .unwrap();
    }

    /// Sends `bytes` as one text frame without checking that they are UTF-8.
    pub(super) async fn send_unchecked_text(&mut self, bytes: &[u8]) {
        let frame = Frame::message(bytes.to_vec(), OpCode::Data(Data::Text), true);
        self.stream.send(Message::Frame(frame)).await.unwrap();
    }

    /// Closes with 1000 and waits for the hub's close.
    pub(super) async fn close(&mut self) -> Option<u16> {
        let normal = CloseFrame {
            code: CloseCode::Normal,
            reason: "".into(),
        };
        self.stream.close(Some(normal)).await.unwrap();
        self.close_code().await
    }
}
