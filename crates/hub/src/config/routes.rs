//! The settings of the public names of applications (HOST-046, task H21). Each is off or empty
//! by default: no deploy opens the interface of the proxy by accident.

use std::{collections::BTreeMap, net::IpAddr, path::PathBuf};

use super::{Error, paths::optional_socket};

/// The Unix socket of the one interface of the proxy process; unset: not served.
const SOCKET: &str = "ANIMA_ROUTES_SOCKET";
/// The user ID of the proxy process: the one peer that the socket answers. Needed with
/// [`SOCKET`] (SF-NEW-revroutesocket).
const PROXY_UID: &str = "ANIMA_ROUTES_PROXY_UID";
/// Comma-separated domains that hold a cookie of a hub: no name at or below them.
const COOKIE_DOMAINS: &str = "ANIMA_ROUTES_COOKIE_DOMAINS";
/// The public address of a hub that a person hosts: a name proves itself by resolving to it.
const PUBLIC_ADDRESS: &str = "ANIMA_ROUTES_PUBLIC_ADDRESS";
/// The public streams at one time on one link.
const STREAMS_PER_LINK: &str = "ANIMA_ROUTES_STREAMS_PER_LINK";
/// The default of [`STREAMS_PER_LINK`].
const STREAMS_DEFAULT: usize = 32;

/// The settings of the routes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Routes {
    pub socket: Option<PathBuf>,
    /// The user ID of the proxy process; set when [`Routes::socket`] is.
    pub proxy_uid: Option<u32>,
    pub cookie_domains: Vec<String>,
    pub public_address: Option<IpAddr>,
    pub streams_per_link: usize,
}

/// Reads the settings of the routes.
///
/// # Errors
/// [`Error::Routes`] for a malformed value or a socket with no user ID of the proxy; the path
/// errors for the socket.
pub(super) fn from_env(env: &BTreeMap<String, String>) -> Result<Routes, Error> {
    let cookie_domains = env.get(COOKIE_DOMAINS).map_or_else(Vec::new, |text| {
        text.split(',')
            .map(|domain| domain.trim().trim_end_matches('.').to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect()
    });
    let public_address = match env.get(PUBLIC_ADDRESS).map(String::as_str) {
        None | Some("") => None,
        Some(text) => Some(text.parse().map_err(|_| Error::Routes {
            variable: PUBLIC_ADDRESS,
        })?),
    };
    let streams_per_link = match env.get(STREAMS_PER_LINK).map(String::as_str) {
        None | Some("") => STREAMS_DEFAULT,
        Some(text) => text
            .parse()
            .ok()
            .filter(|limit| *limit > 0)
            .ok_or(Error::Routes {
                variable: STREAMS_PER_LINK,
            })?,
    };
    let socket = optional_socket(env, SOCKET)?;
    let proxy_uid = match env.get(PROXY_UID).map(String::as_str) {
        None | Some("") => None,
        Some(text) => Some(text.parse().map_err(|_| Error::Routes {
            variable: PROXY_UID,
        })?),
    };
    if socket.is_some() && proxy_uid.is_none() {
        return Err(Error::Routes {
            variable: PROXY_UID,
        });
    }
    Ok(Routes {
        socket,
        proxy_uid,
        cookie_domains,
        public_address,
        streams_per_link,
    })
}
