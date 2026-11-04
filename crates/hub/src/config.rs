//! Hub configuration, parsed from a map of environment variables (no process env, no files).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;

const LISTEN: &str = "ANIMA_LISTEN";
const DATABASE_URL_FILE: &str = "ANIMA_DATABASE_URL_FILE";
const KEY_FILE: &str = "ANIMA_KEY_FILE";
const GUACD: &str = "ANIMA_GUACD";
const PERMITTED_NETWORKS: &str = "ANIMA_PERMITTED_NETWORKS";
const LOG: &str = "ANIMA_LOG";
const TLS_CERT_FILE: &str = "ANIMA_TLS_CERT_FILE";
const TLS_KEY_FILE: &str = "ANIMA_TLS_KEY_FILE";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub listen: SocketAddr,
    pub database_url_file: PathBuf,
    pub key_file: PathBuf,
    /// Gateway `host:port`; the host is a DNS name, so this is not a `SocketAddr`.
    pub guacd: String,
    pub permitted_networks: Vec<ipnet::IpNet>,
    pub log: String,
    pub tls_cert_file: Option<PathBuf>,
    pub tls_key_file: Option<PathBuf>,
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
    #[error("{GUACD}={value:?} is not host:port")]
    Guacd { value: String },
    #[error("{PERMITTED_NETWORKS} has an empty entry")]
    NetworkEmpty,
    #[error("{PERMITTED_NETWORKS} entry {entry:?} is not a CIDR network")]
    Network {
        entry: String,
        #[source]
        source: ipnet::AddrParseError,
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
fn guacd(text: &str) -> Result<String, Error> {
    let invalid = || Error::Guacd {
        value: text.to_owned(),
    };
    let (host, port) = text.rsplit_once(':').ok_or_else(invalid)?;
    let bracketed_v6 = host.starts_with('[') && host.ends_with(']');
    if host.is_empty() || (host.contains(':') && !bracketed_v6) || port.parse::<u16>().is_err() {
        return Err(invalid());
    }
    Ok(text.to_owned())
}

fn networks(text: &str) -> Result<Vec<ipnet::IpNet>, Error> {
    text.split(',')
        .map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return Err(Error::NetworkEmpty);
            }
            entry.parse().map_err(|source| Error::Network {
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
        guacd: guacd(value(env, GUACD, "guacd:4822")?)?,
        permitted_networks: networks(value(env, PERMITTED_NETWORKS, "100.64.0.0/10")?)?,
        log: value(env, LOG, "info")?.to_owned(),
        tls_cert_file,
        tls_key_file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn empty_env_gives_the_documented_defaults() {
        let config = from_env(&env(&[])).unwrap();
        assert_eq!(config.listen, "127.0.0.1:8443".parse().unwrap());
        assert_eq!(
            config.database_url_file,
            PathBuf::from("/run/secrets/database_url")
        );
        assert_eq!(config.key_file, PathBuf::from("/run/secrets/anima_key"));
        assert_eq!(config.guacd, "guacd:4822");
        assert_eq!(
            config.permitted_networks,
            vec!["100.64.0.0/10".parse().unwrap()]
        );
        assert_eq!(config.log, "info");
        assert_eq!(config.tls_cert_file, None);
        assert_eq!(config.tls_key_file, None);
    }

    #[test]
    fn every_variable_overrides_its_default() {
        let config = from_env(&env(&[
            (LISTEN, "10.0.0.1:9000"),
            (DATABASE_URL_FILE, "/db"),
            (KEY_FILE, "/key"),
            (GUACD, "gw.internal:4000"),
            (PERMITTED_NETWORKS, "10.0.0.0/8, 192.168.1.0/24"),
            (LOG, "debug"),
            (TLS_CERT_FILE, "/cert.pem"),
            (TLS_KEY_FILE, "/key.pem"),
        ]))
        .unwrap();
        assert_eq!(config.listen, "10.0.0.1:9000".parse().unwrap());
        assert_eq!(config.database_url_file, PathBuf::from("/db"));
        assert_eq!(config.key_file, PathBuf::from("/key"));
        assert_eq!(config.guacd, "gw.internal:4000");
        assert_eq!(config.permitted_networks.len(), 2);
        assert_eq!(config.log, "debug");
        assert_eq!(config.tls_cert_file, Some(PathBuf::from("/cert.pem")));
        assert_eq!(config.tls_key_file, Some(PathBuf::from("/key.pem")));
    }

    #[test]
    fn invalid_listen_names_the_variable() {
        let error = from_env(&env(&[(LISTEN, "nope")])).unwrap_err();
        assert!(matches!(error, Error::Listen { .. }));
        assert!(error.to_string().contains(LISTEN));
    }

    #[test]
    fn guacd_accepts_bracketed_ipv6_and_rejects_unbracketed() {
        let config = from_env(&env(&[(GUACD, "[::1]:4822")])).unwrap();
        assert_eq!(config.guacd, "[::1]:4822");
        let error = from_env(&env(&[(GUACD, "::1:4822")])).unwrap_err();
        assert!(matches!(error, Error::Guacd { .. }));
    }

    #[test]
    fn empty_tls_files_count_as_unset() {
        let config = from_env(&env(&[(TLS_CERT_FILE, ""), (TLS_KEY_FILE, "")])).unwrap();
        assert_eq!((config.tls_cert_file, config.tls_key_file), (None, None));
    }

    #[test]
    fn collect_env_rejects_non_unicode_anima_and_skips_other_non_unicode() {
        use std::os::unix::ffi::OsStringExt;
        let bad = || OsString::from_vec(vec![0xff, 0xfe]);
        let good = [(OsString::from("ANIMA_LOG"), OsString::from("debug"))];

        let skipped = [
            (OsString::from("OTHER"), bad()),
            (bad(), OsString::from("x")),
        ];
        let map = collect_env(good.clone().into_iter().chain(skipped)).unwrap();
        assert_eq!(map, env(&[(LOG, "debug")]));

        let value = [(OsString::from("ANIMA_LOG"), bad())];
        let error = collect_env(value).unwrap_err();
        assert!(matches!(error, Error::NotUnicode { name } if name == "ANIMA_LOG"));

        let name = [(
            OsString::from_vec(b"ANIMA_\xff".to_vec()),
            OsString::from("x"),
        )];
        assert!(matches!(collect_env(name), Err(Error::NotUnicode { .. })));
    }

    #[test]
    fn guacd_without_host_or_valid_port_is_rejected() {
        for bad in ["guacd", ":4822", "guacd:", "guacd:70000", "guacd:http"] {
            let error = from_env(&env(&[(GUACD, bad)])).unwrap_err();
            assert!(matches!(error, Error::Guacd { .. }), "{bad}");
        }
    }

    #[test]
    fn permitted_networks_reject_empty_entries_and_bad_cidrs() {
        for bad in ["10.0.0.0/8,", ",10.0.0.0/8", "10.0.0.0/8,,192.168.0.0/16"] {
            let error = from_env(&env(&[(PERMITTED_NETWORKS, bad)])).unwrap_err();
            assert!(matches!(error, Error::NetworkEmpty), "{bad}");
        }
        let error = from_env(&env(&[(PERMITTED_NETWORKS, "10.0.0.0/99")])).unwrap_err();
        assert!(matches!(error, Error::Network { .. }));
    }

    #[test]
    fn empty_value_is_rejected_instead_of_defaulted() {
        let error = from_env(&env(&[(LOG, "")])).unwrap_err();
        assert!(matches!(error, Error::Empty { variable: LOG }));
    }

    #[test]
    fn tls_cert_without_key_is_rejected_and_so_is_key_without_cert() {
        let error = from_env(&env(&[(TLS_CERT_FILE, "/c")])).unwrap_err();
        assert!(matches!(
            error,
            Error::TlsIncomplete {
                missing: TLS_KEY_FILE,
                ..
            }
        ));
        let error = from_env(&env(&[(TLS_KEY_FILE, "/k")])).unwrap_err();
        assert!(matches!(
            error,
            Error::TlsIncomplete {
                missing: TLS_CERT_FILE,
                ..
            }
        ));
    }
}
