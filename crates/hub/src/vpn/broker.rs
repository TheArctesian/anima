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
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader},
    net::UnixStream,
    sync::Mutex,
};
use zeroize::Zeroizing;

use super::Error;

/// The longest reply line the hub reads, without its newline (BD-10).
pub(super) const MAX_REPLY_BYTES: usize = 64 * 1024;
/// How long one call may take, from connect to the reply line (BD-10).
const CALL_LIMIT: Duration = Duration::from_secs(30);

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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the key mutations (B4) hand it out")
    )]
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the key mutations (B4) read it")
)]
pub(super) struct NewKey {
    pub key: PreAuthKey,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
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

    /// Every node and the policy sites (`nodes`).
    pub(super) async fn nodes(&self) -> Result<BrokerView, Error> {
        self.call(&json!({ "verb": "nodes" })).await
    }

    /// Sends `request` and reads its reply, holding the turn from connect to close.
    async fn call<T: DeserializeOwned>(&self, request: &Value) -> Result<T, Error> {
        let _turn = self.turn.lock().await;
        let line = tokio::time::timeout(CALL_LIMIT, self.exchange(request))
            .await
            .map_err(|_| Error::BrokerTimedOut)??;
        parse_reply(&line)
    }

    /// One connection: the request line out, the reply line in.
    async fn exchange(&self, request: &Value) -> Result<Zeroizing<Vec<u8>>, Error> {
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
        read_line(stream).await
    }
}

/// The first line of `stream`, at most [`MAX_REPLY_BYTES`]; EOF before any byte, or a longer
/// line, is unreadable.
async fn read_line(stream: UnixStream) -> Result<Zeroizing<Vec<u8>>, Error> {
    let limit = u64::try_from(MAX_REPLY_BYTES + 1).unwrap_or(u64::MAX);
    let mut reader = BufReader::new(stream).take(limit);
    let mut line = Zeroizing::new(Vec::new());
    reader
        .read_until(b'\n', &mut line)
        .await
        .map_err(|_| Error::BrokerUnreadable)?;
    if line.last() == Some(&b'\n') {
        line.pop();
    }
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
