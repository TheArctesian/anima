//! The settings of the proxy, from its environment. The proxy listens only when
//! `ANIMA_ROUTE_PROXY_LISTEN` is set: off by default, so no deploy opens a public listener by
//! accident.

use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf};

use crate::limits::Limits;

const HUB_SOCKET: &str = "ANIMA_ROUTE_PROXY_HUB_SOCKET";
const LISTEN: &str = "ANIMA_ROUTE_PROXY_LISTEN";
const LISTEN_PLAIN: &str = "ANIMA_ROUTE_PROXY_LISTEN_PLAIN";
const DOOR: &str = "ANIMA_ROUTE_PROXY_DOOR";
const CONNECTIONS: &str = "ANIMA_ROUTE_PROXY_CONNECTIONS";
const NEW_PER_SECOND: &str = "ANIMA_ROUTE_PROXY_NEW_PER_SECOND";
const BYTES_PER_DAY: &str = "ANIMA_ROUTE_PROXY_BYTES_PER_DAY";
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
    }))
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
