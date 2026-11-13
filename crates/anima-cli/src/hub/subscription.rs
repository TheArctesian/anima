//! A GraphQL subscription over `graphql-transport-ws` (API-002, K7): the second half of the only
//! network code (CMD-003). A `watch` command opens a typed subscription document and reads its
//! answers one by one:
//!
//! ```ignore
//! let mut events = Subscription::open::<EventAdded>(&hub, event_added::Variables { .. }).await?;
//! while let Some(event) = events.next().await {
//!     print(event?.event_added);
//! }
//! ```
//!
//! The token goes in the `connection_init` payload as `{ "authorization": "Bearer <token>" }`,
//! never in the URL (contracts/conventions.md, "Authentication transport"). Close code 4403 is
//! `UNAUTHENTICATED`. Nothing here logs, and no error holds the token (CMD-014).

use std::marker::PhantomData;

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};
use graphql_client::GraphQLQuery;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as SocketError, Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use zeroize::Zeroizing;

use super::answer;
use super::error::{Code, HubError};
use super::{Hub, Origin, TIMEOUT, Token};

/// The subprotocol name the hub speaks.
const PROTOCOL: &str = "graphql-transport-ws";

/// The close code with which the hub refuses a missing or invalid token (conventions.md).
const CLOSE_FORBIDDEN: u16 = 4403;

/// The status named in a [`HubError::Malformed`] for a frame that is not protocol: the HTTP
/// status of the upgrade.
const UPGRADED: u16 = 101;

/// The identifier of the one operation a socket runs.
const OPERATION_ID: &str = "1";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// An open subscription whose answers are `D`, the typed `data` of its document.
pub struct Subscription<D> {
    link: Link,
    finished: bool,
    answer: PhantomData<fn() -> D>,
}

/// A frame of `graphql-transport-ws` that the client acts on.
#[derive(Debug, PartialEq)]
enum Frame {
    Ack,
    Ping,
    Pong,
    Next(Value),
    Failed(Value),
    Complete,
}

/// The socket and the origin that names it in errors.
struct Link {
    socket: Socket,
    origin: String,
}

impl<D: DeserializeOwned> Subscription<D> {
    /// Connects, signs in with the hub's token and starts document `Q`.
    ///
    /// # Errors
    /// [`HubError::Refused`] (`UNAUTHENTICATED`, `FORBIDDEN`) when the hub does not accept the
    /// token; [`HubError::Unreachable`] when no connection comes in [`TIMEOUT`];
    /// [`HubError::Malformed`] for a hub that does not speak the protocol.
    pub async fn open<Q>(hub: &Hub, variables: Q::Variables) -> Result<Self, HubError>
    where
        Q: GraphQLQuery<ResponseData = D>,
    {
        let body = Q::build_query(variables);
        let origin = hub.origin().as_text();
        let started = tokio::time::timeout(TIMEOUT, start(hub, &body)).await;
        let link =
            started.map_err(|_| unreachable(&origin, "it took longer than 30 seconds"))??;
        Ok(Self {
            link,
            finished: false,
            answer: PhantomData,
        })
    }

    /// The next answer; `None` when the hub completes the subscription. After an error the
    /// subscription is over and this returns `None`.
    pub async fn next(&mut self) -> Option<Result<D, HubError>> {
        while !self.finished {
            let frame = self.link.frame().await;
            match frame {
                Ok(Frame::Next(payload)) => return Some(read(&payload)),
                Ok(Frame::Failed(errors)) => {
                    self.finished = true;
                    return Some(read(&json!({ "errors": errors })));
                }
                Ok(Frame::Complete) => self.finished = true,
                Ok(Frame::Ack | Frame::Ping | Frame::Pong) => {}
                Err(error) => {
                    self.finished = true;
                    return Some(Err(error));
                }
            }
        }
        None
    }
}

async fn start<V: serde::Serialize>(
    hub: &Hub,
    body: &graphql_client::QueryBody<V>,
) -> Result<Link, HubError> {
    let origin = hub.origin().as_text();
    let mut link = Link {
        socket: connect(hub.origin(), &origin).await?,
        origin,
    };
    link.send(init_message(hub.token())).await?;
    while link.frame().await? != Frame::Ack {}
    let subscribe = json!({ "id": OPERATION_ID, "type": "subscribe", "payload": body });
    link.send(Message::text(subscribe.to_string())).await?;
    Ok(link)
}

async fn connect(origin: &Origin, text: &str) -> Result<Socket, HubError> {
    let mut request = socket_url(origin)
        .into_client_request()
        .map_err(|_| HubError::Setup)?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", HeaderValue::from_static(PROTOCOL));
    match connect_async(request).await {
        Ok((socket, _)) => Ok(socket),
        Err(SocketError::Http(response)) => Err(upgrade_refused(response.status().as_u16())),
        Err(error) => Err(unreachable(text, &error.to_string())),
    }
}

impl Link {
    async fn send(&mut self, message: Message) -> Result<(), HubError> {
        self.socket
            .send(message)
            .await
            .map_err(|error| unreachable(&self.origin, &error.to_string()))
    }

    /// The next protocol frame. A ping is answered here and handed on.
    async fn frame(&mut self) -> Result<Frame, HubError> {
        loop {
            let Some(message) = self.socket.next().await else {
                return Err(unreachable(&self.origin, "the connection closed"));
            };
            let message = message.map_err(|error| unreachable(&self.origin, &error.to_string()))?;
            match message {
                Message::Text(text) => match parse(&text)? {
                    Some(Frame::Ping) => {
                        self.send(Message::text(json!({ "type": "pong" }).to_string()))
                            .await?;
                        return Ok(Frame::Ping);
                    }
                    Some(frame) => return Ok(frame),
                    None => {}
                },
                Message::Close(frame) => return Err(closed(&self.origin, frame.as_ref())),
                _ => {}
            }
        }
    }
}

/// `connection_init` with the token. The bytes are wiped when the message drops.
fn init_message(token: Option<&Token>) -> Message {
    let mut bytes = Zeroizing::new(Vec::with_capacity(8 * 1024));
    bytes.extend_from_slice(br#"{"type":"connection_init","payload":"#);
    if let Some(token) = token {
        let text = Zeroizing::new(format!("Bearer {}", token.expose()));
        bytes.extend_from_slice(br#"{"authorization":"#);
        serde_json::to_writer(&mut *bytes, &*text).expect("a string serialises");
        bytes.push(b'}');
    } else {
        bytes.extend_from_slice(b"{}");
    }
    bytes.push(b'}');
    let text = Utf8Bytes::try_from(Bytes::from_owner(bytes)).expect("JSON is UTF-8");
    Message::Text(text)
}

/// `wss://host/graphql` for an `https` hub, `ws://` for plain `http` on this computer.
fn socket_url(origin: &Origin) -> String {
    let mut url = origin.graphql();
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme)
        .expect("http and https change to ws and wss");
    url.to_string()
}

/// The frame a text message holds; `None` for a type this client has no use for.
fn parse(text: &str) -> Result<Option<Frame>, HubError> {
    let malformed = || HubError::Malformed { status: UPGRADED };
    let value: Value = serde_json::from_str(text).map_err(|_| malformed())?;
    let kind = value["type"].as_str().ok_or_else(malformed)?;
    let payload = value["payload"].clone();
    Ok(match kind {
        "connection_ack" => Some(Frame::Ack),
        "ping" => Some(Frame::Ping),
        "pong" => Some(Frame::Pong),
        "next" => Some(Frame::Next(payload)),
        "error" => Some(Frame::Failed(payload)),
        "complete" => Some(Frame::Complete),
        _ => None,
    })
}

/// The typed `data` of a `next` payload, or the first GraphQL error in it.
fn read<D: DeserializeOwned>(payload: &Value) -> Result<D, HubError> {
    let bytes =
        serde_json::to_vec(payload).map_err(|_| HubError::Malformed { status: UPGRADED })?;
    answer::read(UPGRADED, &bytes)
}

fn closed(origin: &str, frame: Option<&CloseFrame>) -> HubError {
    let Some(frame) = frame else {
        return unreachable(origin, "the connection closed");
    };
    let code = u16::from(frame.code);
    if code == CLOSE_FORBIDDEN {
        return HubError::Refused {
            code: Code::Unauthenticated,
            message: "The hub did not accept the token (the socket closed with code 4403)."
                .to_owned(),
            fields: Vec::new(),
        };
    }
    unreachable(origin, &format!("the connection closed with code {code}"))
}

fn upgrade_refused(status: u16) -> HubError {
    let code = match status {
        401 => Code::Unauthenticated,
        403 => Code::Forbidden,
        502..=504 => Code::Unavailable,
        _ => return HubError::Malformed { status },
    };
    HubError::Refused {
        code,
        message: format!("The hub refused the connection (HTTP status {status})."),
        fields: Vec::new(),
    }
}

fn unreachable(origin: &str, reason: &str) -> HubError {
    HubError::Unreachable {
        origin: origin.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    use super::*;

    #[test]
    fn cmd_003_the_socket_url_has_no_token_and_follows_the_scheme() {
        let secure = Origin::parse("https://hub.example:8443").expect("origin");
        assert_eq!(socket_url(&secure), "wss://hub.example:8443/graphql");
        let local = Origin::parse("http://127.0.0.1:9").expect("origin");
        assert_eq!(socket_url(&local), "ws://127.0.0.1:9/graphql");
    }

    #[test]
    fn cmd_003_the_token_is_in_the_init_payload() {
        let token = Token::new(Zeroizing::new("tok-123".to_owned())).expect("token");
        let Message::Text(text) = init_message(Some(&token)) else {
            panic!("a text message");
        };
        let value: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            value,
            json!({ "type": "connection_init", "payload": { "authorization": "Bearer tok-123" } })
        );
    }

    #[test]
    fn cmd_021_close_code_4403_is_unauthenticated_and_others_are_unavailable() {
        let frame = |code: u16| CloseFrame {
            code: CloseCode::from(code),
            reason: Utf8Bytes::from_static(""),
        };
        let refused = closed("https://hub", Some(&frame(4403)));
        assert_eq!(refused.code(), Code::Unauthenticated);
        let lost = closed("https://hub", Some(&frame(1011)));
        assert_eq!(lost.code(), Code::Unavailable);
        assert_eq!(closed("https://hub", None).code(), Code::Unavailable);
    }

    #[test]
    fn cmd_003_frames_are_read_by_type_and_unknown_types_are_ignored() {
        let ack = parse(r#"{"type":"connection_ack"}"#).expect("frame");
        assert_eq!(ack, Some(Frame::Ack));
        let next = parse(r#"{"id":"1","type":"next","payload":{"data":{}}}"#).expect("frame");
        assert_eq!(next, Some(Frame::Next(json!({ "data": {} }))));
        assert_eq!(parse(r#"{"type":"mystery"}"#).expect("frame"), None);
        assert!(parse("not json").is_err());
    }
}
