//! Hub configuration, parsed from a map of environment variables (no process env, no files).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;

mod link;
mod origin;
mod own_daemon;
mod paths;
use paths::{optional_path, optional_root, optional_socket, remote_drive_path};
mod releases;

pub use link::{LINK_DEVICES, dial_socket};
pub use own_daemon::OwnDaemon;
pub use releases::Releases;

const LISTEN: &str = "ANIMA_LISTEN";
const DATABASE_URL_FILE: &str = "ANIMA_DATABASE_URL_FILE";
const KEY_FILE: &str = "ANIMA_KEY_FILE";
const GUACD: &str = "ANIMA_GUACD";
const GUACD_DRIVE_ROOT: &str = "ANIMA_GUACD_DRIVE_ROOT";
const TAILSCALE_LOCALAPI: &str = "ANIMA_TAILSCALE_LOCALAPI";
const VPN_BROKER: &str = "ANIMA_VPN_BROKER";
const PERMITTED_NETWORKS: &str = "ANIMA_PERMITTED_NETWORKS";
const PROBE_DENY: &str = "ANIMA_PROBE_DENY";
const LOG: &str = "ANIMA_LOG";
const TLS_CERT_FILE: &str = "ANIMA_TLS_CERT_FILE";
const TLS_KEY_FILE: &str = "ANIMA_TLS_KEY_FILE";
const PUBLIC_ORIGIN: &str = "ANIMA_PUBLIC_ORIGIN";
const WEB_ROOT: &str = "ANIMA_WEB_ROOT";
const DATA_KEY_DIR: &str = "ANIMA_DATA_KEY_DIR";
const FILES_DIR: &str = "ANIMA_FILES_DIR";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub listen: SocketAddr,
    pub database_url_file: PathBuf,
    pub key_file: PathBuf,
    /// Gateway `host:port`; the host is a DNS name, so this is not a `SocketAddr`.
    pub guacd: String,
    /// Where guacd stores shared drives (RA-021): a path inside the guacd container, not the
    /// hub's filesystem, so it must be absolute.
    pub guacd_drive_root: PathBuf,
    /// `host:port` of the Tailscale `LocalAPI` proxy (D-4). `None` when unset or empty: Tailscale
    /// ping checks then report Unknown.
    pub tailscale_localapi: Option<String>,
    /// The VPN broker's Unix socket on the VPS (R-055, VP-7). `None` when unset or empty:
    /// enrolment is then off and the VPN picture says `NOT_SET`.
    pub vpn_broker: Option<PathBuf>,
    /// In the link mode: this computer and the devices of the link.
    pub permitted_networks: Vec<ipnet::IpNet>,
    /// The Unix socket `anima-link` reaches the hub through; set means the link mode (see
    /// `link`). `None` when unset or empty.
    pub link_socket: Option<PathBuf>,
    /// The control socket of `anima-link hub`: the hub makes invites, lists the link's devices
    /// and revokes one through it (FRN-009). Only in the link mode; `None` when unset or empty.
    pub link_control: Option<PathBuf>,
    /// The file of the Anima.ing cloud's key; set means the hosted mode (hosted-mode.md HMD-037,
    /// R-129): the cloud asks this hub for device invites, and no one signs in with a password.
    /// Only with the link's control socket; `None` when unset or empty.
    pub hosted_cloud_key_file: Option<PathBuf>,
    /// The socket of the hub's own daemon and the file of its code, in the hosted mode only
    /// (HMD-071, HM-T35). `None` when unset or empty.
    pub own_daemon: Option<OwnDaemon>,
    /// Networks checks must never probe: the hub's own Docker networks and public address
    /// (R-030). Empty when unset; `main.rs` warns at start.
    pub probe_deny: Vec<ipnet::IpNet>,
    pub log: String,
    pub tls_cert_file: Option<PathBuf>,
    pub tls_key_file: Option<PathBuf>,
    /// The origin clients use, in canonical form (see `origin`). Compared byte-for-byte with a
    /// browser's `Origin` header (D-3).
    pub public_origin: String,
    /// Whether `ANIMA_PUBLIC_ORIGIN` was given. Without it the origin is the default, which is
    /// right only for a hub that a browser reaches at that address (HMD-070).
    pub public_origin_given: bool,
    /// Folder with the web client's static build. `None` turns the web client off (D-2).
    pub web_root: Option<PathBuf>,
    /// Where releases are published; `None` turns publishing off (R-108).
    pub releases: Option<Releases>,
    /// The folder of the data keys (K-E5 Section 1, VLT-003). `None`: a domain that seals
    /// answers `UNAVAILABLE` and the other domains work.
    pub data_key_dir: Option<PathBuf>,
    /// The folder of the sealed file pieces (K-E4 Section 9, FIL-001). `None`: the hub has no
    /// file store.
    pub files_dir: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{variable} is set but empty")]
    Empty { variable: &'static str },
    #[error("{LISTEN}={value:?} is not a socket address")]
    Listen {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },
    #[error("environment variable {name:?} is not valid unicode")]
    NotUnicode { name: String },
    #[error("{variable}={value:?} is not host:port")]
    HostPort {
        variable: &'static str,
        value: String,
    },
    #[error("{variable}={value:?} is not an absolute path")]
    NotAbsolute {
        variable: &'static str,
        value: String,
    },
    #[error("{variable} has an empty entry")]
    NetworkEmpty { variable: &'static str },
    #[error("{variable} entry {entry:?} is not a CIDR network")]
    Network {
        variable: &'static str,
        entry: String,
        #[source]
        source: ipnet::AddrParseError,
    },
    #[error("{variable}={value:?} is not a canonical origin: {reason}")]
    Origin {
        variable: &'static str,
        value: String,
        reason: &'static str,
    },
    #[error("{variable} is the file-system root, which is too broad to serve or write")]
    FilesystemRoot { variable: &'static str },
    #[error("{variable} has a `..` part, which could climb out of the folder")]
    ParentDirectory { variable: &'static str },
    #[error("{variable}={value:?} is not an R2 bucket name")]
    Bucket {
        variable: &'static str,
        value: String,
    },
    #[error("{LISTEN}={listen} is not a loopback address; {}", link::LISTEN_RULE)]
    LinkListen { listen: SocketAddr },
    #[error("{PERMITTED_NETWORKS} {}", link::NETWORKS_RULE)]
    LinkNetworks,
    #[error("{} {}", link::LINK_CONTROL, link::CONTROL_RULE)]
    LinkControlAlone,
    #[error("{} {}", link::HOSTED_CLOUD_KEY_FILE, link::HOSTED_RULE)]
    HostedWithoutLink,
    #[error("{} {}", own_daemon::OWN_DAEMON_SOCKET, own_daemon::OWN_DAEMON_RULE)]
    OwnDaemonNotHosted,
    #[error("{} {}", own_daemon::OWN_DAEMON_CODE_FILE, own_daemon::CODE_FILE_RULE)]
    OwnDaemonCodeAlone,
    #[error("{present} is set but {missing} is not")]
    TlsIncomplete {
        present: &'static str,
        missing: &'static str,
    },
}

/// Both TLS files or neither: half of the pair is refused.
const fn tls_pair(cert: bool, key: bool) -> Result<(), Error> {
    match (cert, key) {
        (true, false) => Err(Error::TlsIncomplete {
            present: TLS_CERT_FILE,
            missing: TLS_KEY_FILE,
        }),
        (false, true) => Err(Error::TlsIncomplete {
            present: TLS_KEY_FILE,
            missing: TLS_CERT_FILE,
        }),
        _ => Ok(()),
    }
}

/// Read `name`, falling back to `default`; a variable that is set but empty is an error.
fn value<'a>(
    env: &'a BTreeMap<String, String>,
    name: &'static str,
    default: &'a str,
) -> Result<&'a str, Error> {
    match env.get(name).map(String::as_str) {
        Some("") => Err(Error::Empty { variable: name }),
        Some(text) => Ok(text),
        None => Ok(default),
    }
}

/// `host:port` where an IPv6 host must be bracketed.
fn host_port(variable: &'static str, text: &str) -> Result<String, Error> {
    let invalid = || Error::HostPort {
        variable,
        value: text.to_owned(),
    };
    let (host, port) = text.rsplit_once(':').ok_or_else(invalid)?;
    let bracketed_v6 = host.starts_with('[') && host.ends_with(']');
    if host.is_empty() || (host.contains(':') && !bracketed_v6) || port.parse::<u16>().is_err() {
        return Err(invalid());
    }
    Ok(text.to_owned())
}

/// The permitted networks: the link mode's own, or `ANIMA_PERMITTED_NETWORKS`.
fn permitted_networks(
    env: &BTreeMap<String, String>,
    link_mode: bool,
) -> Result<Vec<ipnet::IpNet>, Error> {
    if link_mode {
        return Ok(link::permitted_networks());
    }
    networks(
        PERMITTED_NETWORKS,
        value(env, PERMITTED_NETWORKS, "100.64.0.0/10")?,
    )
}

/// Comma-separated CIDR networks from `variable`.
fn networks(variable: &'static str, text: &str) -> Result<Vec<ipnet::IpNet>, Error> {
    text.split(',')
        .map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return Err(Error::NetworkEmpty { variable });
            }
            entry.parse().map_err(|source| Error::Network {
                variable,
                entry: entry.to_owned(),
                source,
            })
        })
        .collect()
}

/// Collect process variables into a map. A non-unicode `ANIMA_*` name or value is an error;
/// other non-unicode variables are skipped.
///
/// # Errors
/// Returns [`Error::NotUnicode`] naming the offending `ANIMA_*` variable.
pub fn collect_env(
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Result<BTreeMap<String, String>, Error> {
    let mut env = BTreeMap::new();
    for (name, value) in vars {
        let ours = name.as_encoded_bytes().starts_with(b"ANIMA_");
        match (name.into_string(), value.into_string()) {
            (Ok(name), Ok(value)) => {
                env.insert(name, value);
            }
            (name, _) if ours => {
                let name = name.unwrap_or_else(|raw| raw.to_string_lossy().into_owned());
                return Err(Error::NotUnicode { name });
            }
            _ => {}
        }
    }
    Ok(env)
}

/// The release settings alone, for the `anima-hub release` commands, which need no other
/// setting of the hub (R-179); `None` when `ANIMA_RELEASES_ENDPOINT` is unset or empty.
///
/// # Errors
/// Returns the first release variable that is malformed.
pub fn releases_from_env(env: &BTreeMap<String, String>) -> Result<Option<Releases>, Error> {
    releases::from_env(env)
}

/// Build the configuration from `env`, applying defaults for unset variables.
///
/// # Errors
/// Returns the first variable that is empty, malformed, or half of a TLS pair.
pub fn from_env(env: &BTreeMap<String, String>) -> Result<Config, Error> {
    let listen = value(env, LISTEN, "127.0.0.1:8443")?;
    let tls_cert_file = optional_path(env, TLS_CERT_FILE);
    let tls_key_file = optional_path(env, TLS_KEY_FILE);
    tls_pair(tls_cert_file.is_some(), tls_key_file.is_some())?;
    let listen: SocketAddr = listen.parse().map_err(|source| Error::Listen {
        value: listen.to_owned(),
        source,
    })?;
    let link_socket = link::socket(env, listen)?;
    let link_control = link::control(env, link_socket.is_some())?;
    let hosted_cloud_key_file = link::hosted(env, link_control.is_some())?;
    Ok(Config {
        listen,
        database_url_file: value(env, DATABASE_URL_FILE, "/run/secrets/database_url")?.into(),
        key_file: value(env, KEY_FILE, "/run/secrets/anima_key")?.into(),
        guacd: host_port(GUACD, value(env, GUACD, "guacd:4822")?)?,
        guacd_drive_root: remote_drive_path(
            GUACD_DRIVE_ROOT,
            value(env, GUACD_DRIVE_ROOT, "/var/lib/guacd/drives")?,
        )?,
        tailscale_localapi: match env.get(TAILSCALE_LOCALAPI).map(String::as_str) {
            None | Some("") => None,
            Some(text) => Some(host_port(TAILSCALE_LOCALAPI, text)?),
        },
        vpn_broker: optional_socket(env, VPN_BROKER)?,
        permitted_networks: permitted_networks(env, link_socket.is_some())?,
        link_control,
        own_daemon: own_daemon::own_daemon(env, hosted_cloud_key_file.is_some())?,
        hosted_cloud_key_file,
        link_socket,
        probe_deny: match env.get(PROBE_DENY).map(String::as_str) {
            None | Some("") => Vec::new(),
            Some(text) => networks(PROBE_DENY, text)?,
        },
        log: value(env, LOG, "info")?.to_owned(),
        tls_cert_file,
        tls_key_file,
        public_origin: origin::canonical(
            PUBLIC_ORIGIN,
            value(env, PUBLIC_ORIGIN, "https://hub.anima.ing")?,
        )?,
        public_origin_given: env.contains_key(PUBLIC_ORIGIN),
        web_root: optional_root(env, WEB_ROOT)?,
        releases: releases::from_env(env)?,
        data_key_dir: optional_root(env, DATA_KEY_DIR)?,
        files_dir: optional_root(env, FILES_DIR)?,
    })
}

#[cfg(test)]
mod roots_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vpn_tests;
