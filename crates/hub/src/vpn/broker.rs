//! The hub's client of the VPN broker on the VPS (R-055, VP-7, BD-10, BD-15): one JSON
//! request line out, one JSON reply line in, over the broker's Unix socket. The reply is
//! hostile input: it is read to at most 64 KiB, parsed once, and never echoed into an error,
//! because a key reply holds a pre-auth key (LOG-007).

use std::{
    fmt,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
    sync::{Mutex, MutexGuard},
    time::Instant,
};
use zeroize::Zeroizing;

use super::Error;

/// The longest reply line the hub reads, without its newline (BD-10).
pub(super) const MAX_REPLY_BYTES: usize = 64 * 1024;
/// How long the broker waits for its lock behind the node audit before it starts
/// (`flock -w 8` in `deploy/headscale/broker/anima-broker@.unit`).
pub(super) const BROKER_LOCK_WAIT: Duration = Duration::from_secs(8);
/// How long the broker waits for the request line (`READ_MS` in `broker/main.ts`).
pub(super) const BROKER_REQUEST_READ: Duration = Duration::from_secs(5);
/// The broker's request deadline: no Headscale change starts that could end after it
/// (`REQUEST_MS` in `broker/main.ts`, `broker/deadline.ts`).
pub(super) const BROKER_REQUEST_DEADLINE: Duration = Duration::from_secs(45);
/// systemd stops a broker request that runs longer (`RuntimeMaxSec=60` in the unit).
pub(super) const BROKER_RUNTIME_MAX: Duration = Duration::from_secs(60);
/// Time for the reply line to cross the socket after the broker wrote it.
const REPLY_MARGIN: Duration = Duration::from_secs(5);
/// How long one call may take once it has the turn, from connect to the reply line (BD-10):
/// 65 s, longer than a broker request can live, so a change the broker made is never reported
/// as "did not answer", and the turn is never let go while a broker request still runs.
pub(super) const CALL_LIMIT: Duration =
    Duration::from_secs(BROKER_RUNTIME_MAX.as_secs() + REPLY_MARGIN.as_secs());
// The broker's own worst case (lock, request line, deadline) fits in its run time limit.
const _: () = assert!(
    BROKER_LOCK_WAIT.as_secs() + BROKER_REQUEST_READ.as_secs() + BROKER_REQUEST_DEADLINE.as_secs()
        <= BROKER_RUNTIME_MAX.as_secs()
);
/// How long a call waits for the turn behind another call; then it gives up with
/// `BrokerBusy`, having sent nothing (BD-15).
pub(super) const TURN_WAIT: Duration = Duration::from_secs(20);
/// How long a call, holding the turn, waits after the reply line for the broker to close the
/// connection: systemd's `MaxConnections=1` closes the next connection at once while the
/// broker request still runs (BD-15). Never past [`CALL_LIMIT`] from connect.
pub(super) const CLOSE_WAIT: Duration = Duration::from_secs(2);

/// The broker at one socket path. Clones share one turn, so no two calls of the hub reach the
/// socket at the same time: systemd closes a second connection at once (BD-15).
#[derive(Clone, Debug)]
pub struct Broker {
    path: PathBuf,
    turn: Arc<Mutex<()>>,
}

/// One node as the broker lists it, without any key (BD-9).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct BrokerNode {
    pub id: u64,
    pub name: String,
    pub user: String,
    pub tags: Vec<String>,
    pub ips: Vec<IpAddr>,
    pub online: bool,
    pub expiry: Option<DateTime<Utc>>,
    pub approved: Vec<IpNet>,
    pub available: Vec<IpNet>,
}

/// The broker's `nodes` reply: every node, and each policy site by name and subnet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(from = "NodesReply")]
pub(super) struct BrokerView {
    pub nodes: Vec<BrokerNode>,
    pub sites: Vec<(String, IpNet)>,
}

#[derive(Deserialize)]
struct NodesReply {
    nodes: Vec<BrokerNode>,
    sites: Vec<PolicySite>,
}

#[derive(Deserialize)]
struct PolicySite {
    name: String,
    subnet: IpNet,
}

impl From<NodesReply> for BrokerView {
    fn from(reply: NodesReply) -> Self {
        Self {
            nodes: reply.nodes,
            sites: reply
                .sites
                .into_iter()
                .map(|site| (site.name, site.subnet))
                .collect(),
        }
    }
}

/// A one-use pre-auth key. It has no `Display`, its `Debug` prints no key, and its memory is
/// zeroed when it drops (LOG-007).
pub(super) struct PreAuthKey(Zeroizing<String>);

impl PreAuthKey {
    /// The key, for the one reply that hands it to the operator.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PreAuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[pre-auth key]")
    }
}

impl<'de> Deserialize<'de> for PreAuthKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|key| Self(Zeroizing::new(key)))
    }
}

/// The broker's reply to `personal-key` and `router-key`.
#[derive(Debug, Deserialize)]
pub(super) struct NewKey {
    pub key: PreAuthKey,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// The broker's reply to `register` and `renew`: the node's name, IPv4 address and expiry.
#[derive(Debug, Deserialize)]
pub(super) struct Registered {
    pub name: String,
    pub address: Ipv4Addr,
    #[serde(rename = "keyExpiry")]
    pub key_expiry: Option<DateTime<Utc>>,
}

/// Why the broker refused a request; the words of its wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    BadRequest,
    UnknownSite,
    RateLimited,
    NodeLimit,
    NotAllowed,
    Undone,
    HeadscaleFailed,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadRequest => "bad-request",
            Self::UnknownSite => "unknown-site",
            Self::RateLimited => "rate-limited",
            Self::NodeLimit => "node-limit",
            Self::NotAllowed => "not-allowed",
            Self::Undone => "undone",
            Self::HeadscaleFailed => "headscale-failed",
        })
    }
}

/// One reply line: `{"ok":true,"value":…}` or `{"ok":false,"refusal":…,"detail":…}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply<T> {
    ok: bool,
    value: Option<T>,
    refusal: Option<Refusal>,
    #[serde(default)]
    detail: String,
}

impl Broker {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            turn: Arc::new(Mutex::new(())),
        }
    }

    /// Every node and the policy sites (`nodes`); `BrokerBusy` at once when another call holds
    /// the turn, so the reader never queues behind a mutation (BD-15).
    pub(super) async fn nodes_if_free(&self) -> Result<BrokerView, Error> {
        let turn = self.turn.try_lock().map_err(|_| Error::BrokerBusy)?;
        self.call_holding(turn, &json!({ "verb": "nodes" })).await
    }

    /// A one-use, one-hour key for an untagged node of the user `operator` (VPN-003).
    pub(super) async fn personal_key(&self) -> Result<NewKey, Error> {
        self.call(&json!({ "verb": "personal-key" })).await
    }

    /// A one-use, one-hour key tagged for the router of the policy site `site` (VPN-001).
    pub(super) async fn router_key(&self, site: &str) -> Result<NewKey, Error> {
        self.call(&json!({ "verb": "router-key", "site": site }))
            .await
    }

    /// Registers the node that waits with `auth_id` to the user `operator` (vpn.md 6B).
    pub(super) async fn register(&self, auth_id: &str) -> Result<Registered, Error> {
        self.call(&json!({ "verb": "register", "authId": auth_id }))
            .await
    }

    /// Approves the one subnet of the policy site `site` that router `node` offers (R-055).
    pub(super) async fn approve_route(&self, node: u64, site: &str) -> Result<(), Error> {
        let request = json!({ "verb": "approve-route", "node": node, "site": site });
        self.call::<serde::de::IgnoredAny>(&request).await.map(drop)
    }

    /// Sets a staggered key expiry on the untagged operator node `node` (BD-17, VP-10).
    pub(super) async fn renew(&self, node: u64) -> Result<Registered, Error> {
        self.call(&json!({ "verb": "renew", "node": node })).await
    }

    /// Waits at most [`TURN_WAIT`] for the turn, then sends `request` and reads its reply.
    async fn call<T: DeserializeOwned>(&self, request: &Value) -> Result<T, Error> {
        let turn = tokio::time::timeout(TURN_WAIT, self.turn.lock())
            .await
            .map_err(|_| Error::BrokerBusy)?;
        self.call_holding(turn, request).await
    }

    /// Sends `request` and reads its reply within [`CALL_LIMIT`], holding `_turn` from connect
    /// to close.
    async fn call_holding<T: DeserializeOwned>(
        &self,
        _turn: MutexGuard<'_, ()>,
        request: &Value,
    ) -> Result<T, Error> {
        let deadline = Instant::now() + CALL_LIMIT;
        let (line, stream) = tokio::time::timeout_at(deadline, self.exchange(request))
            .await
            .map_err(|_| Error::BrokerTimedOut)??;
        // The reply is in hand: a broker that does not close in time fails nothing.
        let closed_by = deadline.min(Instant::now() + CLOSE_WAIT);
        let _ = tokio::time::timeout_at(closed_by, closed(stream)).await;
        parse_reply(&line)
    }

    /// One connection: the request line out, the reply line in, and the connection to wait on.
    async fn exchange(&self, request: &Value) -> Result<(Zeroizing<Vec<u8>>, UnixStream), Error> {
        let mut stream =
            UnixStream::connect(&self.path)
                .await
                .map_err(|error| match error.kind() {
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused => Error::NotInstalled,
                    _ => Error::BrokerConnection,
                })?;
        let mut out = request.to_string().into_bytes();
        out.push(b'\n');
        stream
            .write_all(&out)
            .await
            .map_err(|_| Error::BrokerConnection)?;
        let line = read_line(&mut stream).await?;
        Ok((line, stream))
    }
}

/// Reads and drops whatever follows the reply line until the broker closes the connection
/// (EOF) or it fails; the bytes go into a small buffer that is zeroed when it drops.
async fn closed(mut stream: UnixStream) {
    let mut rest = Zeroizing::new([0_u8; 512]);
    while matches!(stream.read(&mut rest[..]).await, Ok(read) if read > 0) {}
}

/// The first line of `stream`, at most [`MAX_REPLY_BYTES`]; EOF before any byte, or a longer
/// line, is unreadable. The bytes go from the socket straight into one buffer of the full size,
/// which never grows and is zeroed when it drops, so no copy of a key is left (LOG-007).
async fn read_line(stream: &mut UnixStream) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut line = Zeroizing::new(vec![0_u8; MAX_REPLY_BYTES + 1]);
    let mut filled = 0;
    let end = loop {
        let read = stream
            .read(&mut line[filled..])
            .await
            .map_err(|_| Error::BrokerUnreadable)?;
        let newline = line[filled..filled + read].iter().position(|&b| b == b'\n');
        filled += read;
        match newline {
            Some(at) => break filled - read + at,
            None if read == 0 || filled == line.len() => break filled,
            None => {}
        }
    };
    line.truncate(end);
    if line.is_empty() || line.len() > MAX_REPLY_BYTES {
        return Err(Error::BrokerUnreadable);
    }
    Ok(line)
}

/// The value of one reply line, or the broker's refusal. A parse error never carries the
/// line's text: serde quotes the value it could not read, and that can be a key (LOG-007).
pub(super) fn parse_reply<T: DeserializeOwned>(line: &[u8]) -> Result<T, Error> {
    let reply: Reply<T> = serde_json::from_slice(line).map_err(|_| Error::BrokerUnreadable)?;
    match (reply.ok, reply.value, reply.refusal) {
        (true, Some(value), None) => Ok(value),
        (false, None, Some(refusal)) => Err(Error::BrokerRefused {
            refusal,
            detail: crate::text::clean(&reply.detail, DETAIL_CHARS),
        }),
        _ => Err(Error::BrokerUnreadable),
    }
}

/// The longest refusal detail the hub keeps.
const DETAIL_CHARS: usize = 160;

/// The first IPv4 address of a broker node, as the picture matches nodes by it (BD-14).
pub(super) fn ipv4_of(node: &BrokerNode) -> Option<Ipv4Addr> {
    node.ips.iter().find_map(|ip| match ip {
        IpAddr::V4(v4) => Some(*v4),
        IpAddr::V6(_) => None,
    })
}
