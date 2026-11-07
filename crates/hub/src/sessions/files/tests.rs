//! Tests of the files tunnel (K4 Sections 7 and 9): the messages, property-tested
//! (`protocol`), the byte counts (`transfer`), the tickets (`tickets`), and over real sockets
//! against a test SSH server with an SFTP subsystem (`opening`, `route`, `transfers`,
//! `refusals`, `ending`). In-crate tests, so the waits and the loopback deny list stay
//! crate-private.

mod bench;
mod ending;
mod opening;
mod protocol;
mod refusals;
mod route;
mod server;
mod sftp;
mod socket;
mod tickets;
mod transfer;
mod transfers;

use std::time::Duration;

/// The longest a test waits for any one thing.
const WAIT: Duration = Duration::from_secs(5);

/// `text` as a client path, as the parser accepts it.
fn device_path(text: &str) -> super::protocol::DevicePath {
    let message = format!(r#"{{"type":"list","path":{}}}"#, serde_json::json!(text));
    match super::protocol::parse(&message) {
        Ok(super::protocol::Request::List(path)) => path,
        other => panic!("{other:?}"),
    }
}
