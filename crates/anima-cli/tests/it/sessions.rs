//! `anima session`, `anima ssh` and `anima files` against a scripted hub with tunnels (K4).
//! The interactive path (raw mode, window changes) needs a real terminal and is not run here;
//! `ssh` is tested with standard input from a pipe, which takes the same relay.

mod files;
mod list;
mod terminal;

use serde_json::{Value, json};

use crate::kit::fake_hub::data;
use crate::kit::tunnel_hub::{FILES_TICKET, TERMINAL_TICKET};

pub const LOGIN_TOKEN: &str = "login-token-for-session-tests";
pub const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
pub const SESSION: &str = "0190a000-0000-7000-8009-000000000001";

/// A K1 `Session` as the documents select it.
pub fn session(reason: Option<&str>) -> Value {
    json!({
        "id": SESSION, "deviceId": DEVICE, "deviceName": "web-1", "protocol": "SSH",
        "operatorName": "operator", "startedAt": "2026-11-13T09:00:00Z",
        "endedAt": reason.map(|_| "2026-11-13T09:05:00Z"), "endReason": reason,
        "clientAddr": "10.10.0.5",
    })
}

/// The `startSession` answer with the terminal ticket.
pub fn started(wake_sent: bool) -> Value {
    data(json!({ "startSession": {
        "sessionId": SESSION, "token": TERMINAL_TICKET, "tunnelPath": "/terminal",
        "wakeSent": wake_sent,
    }}))
}

/// The `openFiles` answer with the files ticket.
pub fn files_opened() -> Value {
    data(json!({ "openFiles": { "token": FILES_TICKET, "tunnelPath": "/files" } }))
}

/// The `endSession` answer, ended with `reason`.
pub fn ended(reason: &str) -> Value {
    data(json!({ "endSession": session(Some(reason)) }))
}
