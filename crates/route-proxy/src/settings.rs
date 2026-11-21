//! The settings of the proxy, from its environment. The proxy listens only when
//! `ANIMA_ROUTE_PROXY_LISTEN` is set: off by default, so no deploy opens a public listener by
//! accident.

use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, time::Duration};

use crate::limits::{Caps, Limits, Timeouts};

const HUB_SOCKET: &str = "ANIMA_ROUTE_PROXY_HUB_SOCKET";
const LISTEN: &str = "ANIMA_ROUTE_PROXY_LISTEN";
const LISTEN_PLAIN: &str = "ANIMA_ROUTE_PROXY_LISTEN_PLAIN";
const DOOR: &str = "ANIMA_ROUTE_PROXY_DOOR";
const CONNECTIONS: &str = "ANIMA_ROUTE_PROXY_CONNECTIONS";
const NEW_PER_SECOND: &str = "ANIMA_ROUTE_PROXY_NEW_PER_SECOND";
const BYTES_PER_DAY: &str = "ANIMA_ROUTE_PROXY_BYTES_PER_DAY";
const HANDSHAKE_SECONDS: &str = "ANIMA_ROUTE_PROXY_HANDSHAKE_SECONDS";
const HEAD_SECONDS: &str = "ANIMA_ROUTE_PROXY_HEAD_SECONDS";
const IDLE_SECONDS: &str = "ANIMA_ROUTE_PROXY_IDLE_SECONDS";
const OPEN_TOTAL: &str = "ANIMA_ROUTE_PROXY_OPEN_TOTAL";
const OPEN_PER_CLIENT: &str = "ANIMA_ROUTE_PROXY_OPEN_PER_CLIENT";
const OPEN_PLAIN: &str = "ANIMA_ROUTE_PROXY_OPEN_PLAIN";
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// What the proxy runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub hub_socket: PathBuf,
    /// The TLS listener (443 on a hub that a person hosts).
    pub listen: SocketAddr,
    /// The plain HTTP listener for ACME and the redirect (80); `None`: not served.
    pub listen_plain: Option<SocketAddr>,
    /// The connections come from the door with a PROXY header. Never on a self-hosted hub.
    pub door: bool,
    pub limits: Limits,
    pub timeouts: Timeouts,
    /// The connections of the TLS listener before the name is known.
    pub tls_caps: Caps,
    /// The connections of the plain listener.
    pub plain_caps: Caps,
}

/// Why the settings were not read.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("{0} is not valid")]
    Invalid(&'static str),
    #[error("{HUB_SOCKET} is not set")]
    NoHub,
}

/// The settings, or `None` when the proxy is off.
///
/// # Errors
/// [`SettingsError`] for a malformed or missing value.
pub fn from_env(env: &BTreeMap<String, String>) -> Result<Option<Settings>, SettingsError> {
    let Some(listen) = read::<SocketAddr>(env, LISTEN)? else {
        return Ok(None);
    };
    let hub_socket = env
        .get(HUB_SOCKET)
        .filter(|path| path.starts_with('/'))
        .map(PathBuf::from)
        .ok_or(SettingsError::NoHub)?;
    Ok(Some(Settings {
        hub_socket,
        listen,
        listen_plain: read(env, LISTEN_PLAIN)?,
        door: read::<bool>(env, DOOR)?.unwrap_or(false),
        limits: Limits {
            connections: read(env, CONNECTIONS)?.unwrap_or(64),
            new_per_second: read(env, NEW_PER_SECOND)?.unwrap_or(20),
            bytes_per_period: read(env, BYTES_PER_DAY)?.unwrap_or(0),
            period_ms: DAY_MS,
        },
        timeouts: timeouts(env)?,
        tls_caps: Caps {
            total: positive(env, OPEN_TOTAL)?.unwrap_or(1024),
            per_client: positive(env, OPEN_PER_CLIENT)?.unwrap_or(32),
        },
        plain_caps: Caps {
            total: positive(env, OPEN_PLAIN)?.unwrap_or(256),
            per_client: positive(env, OPEN_PER_CLIENT)?.unwrap_or(32),
        },
    }))
}

/// The timeouts: 10 s for the handshake and the head, 300 s with no byte by default.
fn timeouts(env: &BTreeMap<String, String>) -> Result<Timeouts, SettingsError> {
    let default = Timeouts::default();
    let seconds = |name, default: Duration| {
        positive::<u64>(env, name).map(|value| value.map_or(default, Duration::from_secs))
    };
    Ok(Timeouts {
        handshake: seconds(HANDSHAKE_SECONDS, default.handshake)?,
        head: seconds(HEAD_SECONDS, default.head)?,
        idle: seconds(IDLE_SECONDS, default.idle)?,
    })
}

/// A number above 0, or `None` when it is not set.
fn positive<T: std::str::FromStr + Default + PartialEq>(
    env: &BTreeMap<String, String>,
    name: &'static str,
) -> Result<Option<T>, SettingsError> {
    match read::<T>(env, name)? {
        Some(value) if value == T::default() => Err(SettingsError::Invalid(name)),
        value => Ok(value),
    }
}

fn read<T: std::str::FromStr>(
    env: &BTreeMap<String, String>,
    name: &'static str,
) -> Result<Option<T>, SettingsError> {
    match env.get(name).map(String::as_str) {
        None | Some("") => Ok(None),
        Some(text) => text
            .parse()
            .map(Some)
            .map_err(|_| SettingsError::Invalid(name)),
    }
}
