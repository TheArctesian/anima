//! The operator's client as guacamole-common-js opens it: a WebSocket to `/tunnel?token=…`
//! with the subprotocol `guacamole`, one instruction per text message.

use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
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

/// The longest the client waits for any one message.
const WAIT: Duration = Duration::from_secs(5);

/// What the client got next.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Received {
    Text(String),
    Closed(Option<u16>),
}

/// An open tunnel from the client's side. `seen` keeps every text message, so a test can
/// check what never reached the client.
pub(super) struct Socket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub(super) seen: Vec<String>,
}

/// Opens the tunnel at `url`, offering `guacamole` when `subprotocol` is set. A refusal
/// before the upgrade gives its HTTP status.
pub(super) async fn open_with(url: &str, subprotocol: bool) -> Result<Socket, StatusCode> {
    let mut request = url.into_client_request().unwrap();
    if subprotocol {
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "guacamole".parse().unwrap());
    }
    match timeout(WAIT, connect_async(request)).await.unwrap() {
        Ok((stream, response)) => {
            assert_eq!(
                response.headers()["Sec-WebSocket-Protocol"],
                "guacamole",
                "the hub echoes the subprotocol"
            );
            Ok(Socket {
                stream,
                seen: Vec::new(),
            })
        }
        Err(Error::Http(response)) => {
            assert!(
                response.body().as_ref().is_none_or(Vec::is_empty),
                "a refusal has no body detail"
            );
            Err(response.status())
        }
        Err(other) => panic!("the tunnel request failed: {other}"),
    }
}

/// Opens the tunnel as guacamole-common-js does.
pub(super) async fn open(url: &str) -> Socket {
    open_with(url, true).await.expect("the tunnel opens")
}

impl Socket {
    /// The next text message or the close, skipping pings and pongs.
    pub(super) async fn next(&mut self) -> Received {
        loop {
            let message = timeout(WAIT, self.stream.next()).await.expect("a message");
            match message {
                Some(Ok(Message::Text(text))) => {
                    self.seen.push(text.to_string());
                    return Received::Text(text.to_string());
                }
                Some(Ok(Message::Close(frame))) => {
                    return Received::Closed(frame.map(|frame| u16::from(frame.code)));
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Binary(_))) => panic!("the hub sends text only"),
                Some(Err(_)) | None => return Received::Closed(None),
            }
        }
    }

    /// The next text message.
    pub(super) async fn text(&mut self) -> String {
        match self.next().await {
            Received::Text(text) => text,
            Received::Closed(code) => panic!("closed with {code:?} while a text was due"),
        }
    }

    /// The close code, after any text messages still on the way.
    pub(super) async fn close_code(&mut self) -> Option<u16> {
        loop {
            if let Received::Closed(code) = self.next().await {
                return code;
            }
        }
    }

    pub(super) async fn send(&mut self, text: &str) {
        self.stream.send(Message::text(text)).await.unwrap();
    }

    /// Sends `bytes` as one text frame without checking that they are UTF-8.
    pub(super) async fn send_unchecked_text(&mut self, bytes: &[u8]) {
        let frame = Frame::message(bytes.to_vec(), OpCode::Data(Data::Text), true);
        self.stream.send(Message::Frame(frame)).await.unwrap();
    }

    /// Sends `text` up to `count` times, until a send fails or stalls for a second.
    pub(super) async fn flood(&mut self, text: &str, count: usize) {
        for _ in 0..count {
            let sent = timeout(WAIT / 5, self.stream.send(Message::text(text))).await;
            if !matches!(sent, Ok(Ok(()))) {
                return;
            }
        }
    }

    pub(super) async fn send_binary(&mut self, bytes: &[u8]) {
        self.stream
            .send(Message::binary(bytes.to_vec()))
            .await
            .unwrap();
    }

    /// Closes the WebSocket with 1000, as a closing browser tab does, and waits for the hub's
    /// close, which echoes the code (RFC 6455 Section 5.5.1).
    pub(super) async fn close(&mut self) -> Option<u16> {
        let normal = CloseFrame {
            code: CloseCode::Normal,
            reason: "".into(),
        };
        self.stream.close(Some(normal)).await.unwrap();
        self.close_code().await
    }

    /// Drops the connection without a close frame, as a lost network does.
    pub(super) fn drop_connection(self) {
        drop(self.stream);
    }
}
