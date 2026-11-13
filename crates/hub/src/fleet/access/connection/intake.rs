//! What a client sends on one connection, read before the protocol sees it: a read error or an
//! operation over the limits ends the input and names the close (H-6(b), API-004). At most
//! [`MAX_OPERATIONS`] operations run at once, and each id once.

use std::{
    collections::HashSet,
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use async_graphql::{
    futures_util::{Stream, StreamExt as _, future},
    http::WsMessage,
};
use axum::{body::Bytes, extract::ws::Message};
use serde::Deserialize;

use super::Closing;
use crate::fleet::access::lock;

/// The client's side of one connection, shared by the input stream and the relay. The locks
/// are held for one map update, never across an await.
#[derive(Clone, Default)]
pub(super) struct Intake {
    live: Arc<Mutex<Live>>,
    ended: Arc<Mutex<Option<Closing>>>,
}

impl Intake {
    /// The client's text and binary messages in order. The stream ends at the first read error
    /// or overrun, and [`Intake::reason`] then names the close.
    pub(super) fn read(
        self,
        source: impl Stream<Item = Result<Message, axum::Error>>,
    ) -> impl Stream<Item = Bytes> {
        source
            .map(move |read| self.take(read))
            .take_while(|kept| future::ready(kept.is_continue()))
            .filter_map(|kept| future::ready(kept.continue_value().flatten()))
    }

    /// `Break` to end the input; `Continue(None)` to skip a control message.
    fn take(&self, read: Result<Message, axum::Error>) -> ControlFlow<(), Option<Bytes>> {
        let Ok(message) = read else {
            return self.end(Closing::Unreadable);
        };
        let (Message::Text(_) | Message::Binary(_)) = message else {
            return ControlFlow::Continue(None);
        };
        let bytes = message.into_data();
        match lock(&self.live).admit(&bytes) {
            Ok(()) => ControlFlow::Continue(Some(bytes)),
            Err(Overrun::Duplicate) => self.end(Closing::DuplicateOperation),
            Err(Overrun::TooMany) => self.end(Closing::TooManyOperations),
        }
    }

    fn end(&self, closing: Closing) -> ControlFlow<(), Option<Bytes>> {
        *lock(&self.ended) = Some(closing);
        ControlFlow::Break(())
    }

    /// Notes a message the hub sends; a `complete` ends its operation.
    pub(super) fn sent(&self, message: &WsMessage) {
        if let WsMessage::Text(text) = message {
            lock(&self.live).ended(text);
        }
    }

    /// Why the connection closes: when the protocol saw the input end, the reason the input
    /// ended, else `closing`.
    pub(super) fn reason(&self, closing: Closing) -> Closing {
        match closing {
            Closing::ClientLeft => lock(&self.ended).take().unwrap_or(closing),
            other => other,
        }
    }
}

/// The most operations one connection may have running at once.
pub(super) const MAX_OPERATIONS: usize = 32;

/// The part of a protocol message that names an operation.
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: String,
    id: Option<String>,
}

/// Why a client message is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Overrun {
    /// A `subscribe` reuses the id of a running operation.
    Duplicate,
    /// A `subscribe` would run more than [`MAX_OPERATIONS`] at once.
    TooMany,
}

/// The ids of the running operations.
#[derive(Debug, Default)]
pub(super) struct Live {
    ids: HashSet<String>,
}

impl Live {
    /// Counts one client message: a `subscribe` starts an operation, a `complete` ends one.
    /// async-graphql also reads `start` and `stop` as those two, so they count the same. A
    /// message that does not parse passes; the protocol refuses it.
    pub(super) fn admit(&mut self, message: &[u8]) -> Result<(), Overrun> {
        let Ok(Envelope { kind, id: Some(id) }) = serde_json::from_slice(message) else {
            return Ok(());
        };
        match kind.as_str() {
            "subscribe" | "start" if self.ids.contains(&id) => Err(Overrun::Duplicate),
            "subscribe" | "start" if self.ids.len() >= MAX_OPERATIONS => Err(Overrun::TooMany),
            "subscribe" | "start" => {
                self.ids.insert(id);
                Ok(())
            }
            "complete" | "stop" => {
                self.ids.remove(&id);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Forgets the operation that a `complete` the hub sends ends.
    pub(super) fn ended(&mut self, sent: &str) {
        if !sent.starts_with(r#"{"type":"complete""#) {
            return;
        }
        if let Ok(Envelope { id: Some(id), .. }) = serde_json::from_str(sent) {
            self.ids.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Live, MAX_OPERATIONS, Overrun};

    fn subscribe(id: &str) -> Vec<u8> {
        format!(r#"{{"id":"{id}","type":"subscribe","payload":{{"query":"{{ a }}"}}}}"#)
            .into_bytes()
    }

    #[test]
    fn h_6_a_connection_runs_at_most_32_operations() {
        let mut live = Live::default();
        for id in 0..MAX_OPERATIONS {
            assert_eq!(live.admit(&subscribe(&id.to_string())), Ok(()), "{id}");
        }
        assert_eq!(live.admit(&subscribe("one-more")), Err(Overrun::TooMany));
        assert_eq!(
            live.admit(br#"{"type":"start","id":"x","payload":{}}"#),
            Err(Overrun::TooMany)
        );
        assert_eq!(live.admit(br#"{"type":"complete","id":"3"}"#), Ok(()));
        assert_eq!(live.admit(&subscribe("one-more")), Ok(()));
        live.ended(r#"{"type":"complete","id":"4"}"#);
        assert_eq!(live.admit(&subscribe("another")), Ok(()));
        assert_eq!(live.admit(&subscribe("last")), Err(Overrun::TooMany));
    }

    #[test]
    fn h_6_an_id_runs_once_at_a_time() {
        let mut live = Live::default();
        assert_eq!(live.admit(&subscribe("1")), Ok(()));
        assert_eq!(live.admit(&subscribe("1")), Err(Overrun::Duplicate));
        assert_eq!(
            live.admit(br#"{"type":"start","id":"1","payload":{}}"#),
            Err(Overrun::Duplicate)
        );
        assert_eq!(live.admit(br#"{"type":"stop","id":"1"}"#), Ok(()));
        assert_eq!(live.admit(&subscribe("1")), Ok(()));
        live.ended(r#"{"type":"next","id":"1","payload":{}}"#);
        assert_eq!(live.admit(&subscribe("1")), Err(Overrun::Duplicate));
        live.ended(r#"{"type":"complete","id":"1"}"#);
        assert_eq!(live.admit(&subscribe("1")), Ok(()));
    }

    #[test]
    fn h_6_other_messages_do_not_count() {
        let mut live = Live::default();
        for other in [
            &br#"{"type":"connection_init","payload":{}}"#[..],
            br#"{"type":"ping"}"#,
            br#"{"type":"subscribe"}"#,
            b"not json",
        ] {
            assert_eq!(live.admit(other), Ok(()));
        }
        assert!(live.ids.is_empty());
    }
}
