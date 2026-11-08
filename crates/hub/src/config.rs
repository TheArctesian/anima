//! Hub configuration, parsed from a map of environment variables (no process env, no files).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;

mod origin;

const LISTEN: &str = "ANIMA_LISTEN";
const DATABASE_URL_FILE: &str = "ANIMA_DATABASE_URL_FILE";
const KEY_FILE: &str = "ANIMA_KEY_FILE";
const GUACD: &str = "ANIMA_GUACD";
const GUACD_DRIVE_ROOT: &str = "ANIMA_GUACD_DRIVE_ROOT";
const TAILSCALE_LOCALAPI: &str = "ANIMA_TAILSCALE_LOCALAPI";
const PERMITTED_NETWORKS: &str = "ANIMA_PERMITTED_NETWORKS";
const PROBE_DENY: &str = "ANIMA_PROBE_DENY";
const LOG: &str = "ANIMA_LOG";
const TLS_CERT_FILE: &str = "ANIMA_TLS_CERT_FILE";
const TLS_KEY_FILE: &str = "ANIMA_TLS_KEY_FILE";
const PUBLIC_ORIGIN: &str = "ANIMA_PUBLIC_ORIGIN";
const WEB_ROOT: &str = "ANIMA_WEB_ROOT";
const RECORDING_ROOT: &str = "ANIMA_RECORDING_ROOT";
const RECORDING_MAX_BYTES: &str = "ANIMA_RECORDING_MAX_BYTES";

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
    pub permitted_networks: Vec<ipnet::IpNet>,
    /// Networks checks must never probe: the hub's own Docker networks and public address
    /// (R-030). Empty when unset; `main.rs` warns at start.
    pub probe_deny: Vec<ipnet::IpNet>,
    pub log: String,
    pub tls_cert_file: Option<PathBuf>,
    pub tls_key_file: Option<PathBuf>,
    /// The origin clients use, in canonical form (see `origin`). Compared byte-for-byte with a
    /// browser's `Origin` header (D-3).
    pub public_origin: String,
    /// Folder with the web client's static build. `None` turns the web client off (D-2).
    pub web_root: Option<PathBuf>,
    /// Folder for recording files. `None`: recording is unavailable.
    pub recording_root: Option<PathBuf>,
    /// Plaintext byte limit of one recording; always positive.
    pub recording_max_bytes: u64,
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
    #[error("{variable}={value:?} is not a positive integer")]
    NotPositive {
        variable: &'static str,
        value: String,
    },
    #[error("{present} is set but {missing} is not")]
    TlsIncomplete {
        present: &'static str,
        missing: &'static str,
    },
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

/// An empty value (Compose passes `VAR=`) counts as unset.
fn optional_path(env: &BTreeMap<String, String>, name: &str) -> Option<PathBuf> {
    env.get(name)
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
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

/// A path in another filesystem (the guacd container), which only an absolute path names.
fn absolute_path(variable: &'static str, text: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(text);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(Error::NotAbsolute {
            variable,
            value: text.to_owned(),
        })
    }
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

/// A positive `u64`: zero, negative and non-numeric text are refused.
fn positive_u64(variable: &'static str, text: &str) -> Result<u64, Error> {
    match text.parse::<u64>() {
        Ok(number) if number > 0 => Ok(number),
        _ => Err(Error::NotPositive {
            variable,
            value: text.to_owned(),
        }),
    }
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

/// Build the configuration from `env`, applying defaults for unset variables.
///
/// # Errors
/// Returns the first variable that is empty, malformed, or half of a TLS pair.
pub fn from_env(env: &BTreeMap<String, String>) -> Result<Config, Error> {
    let listen = value(env, LISTEN, "127.0.0.1:8443")?;
    let tls_cert_file = optional_path(env, TLS_CERT_FILE);
    let tls_key_file = optional_path(env, TLS_KEY_FILE);
    match (&tls_cert_file, &tls_key_file) {
        (Some(_), None) => {
            return Err(Error::TlsIncomplete {
                present: TLS_CERT_FILE,
                missing: TLS_KEY_FILE,
            });
        }
        (None, Some(_)) => {
            return Err(Error::TlsIncomplete {
                present: TLS_KEY_FILE,
                missing: TLS_CERT_FILE,
            });
        }
        _ => {}
    }
    Ok(Config {
        listen: listen.parse().map_err(|source| Error::Listen {
            value: listen.to_owned(),
            source,
        })?,
        database_url_file: value(env, DATABASE_URL_FILE, "/run/secrets/database_url")?.into(),
        key_file: value(env, KEY_FILE, "/run/secrets/anima_key")?.into(),
        guacd: host_port(GUACD, value(env, GUACD, "guacd:4822")?)?,
        guacd_drive_root: absolute_path(
            GUACD_DRIVE_ROOT,
            value(env, GUACD_DRIVE_ROOT, "/var/lib/guacd/drives")?,
        )?,
        tailscale_localapi: match env.get(TAILSCALE_LOCALAPI).map(String::as_str) {
            None | Some("") => None,
            Some(text) => Some(host_port(TAILSCALE_LOCALAPI, text)?),
        },
        permitted_networks: networks(
            PERMITTED_NETWORKS,
            value(env, PERMITTED_NETWORKS, "100.64.0.0/10")?,
        )?,
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
        web_root: optional_path(env, WEB_ROOT),
        recording_root: optional_path(env, RECORDING_ROOT),
        recording_max_bytes: positive_u64(
            RECORDING_MAX_BYTES,
            value(env, RECORDING_MAX_BYTES, "2147483648")?,
        )?,
    })
}

#[cfg(test)]
mod k7_tests;
#[cfg(test)]
mod tests;
