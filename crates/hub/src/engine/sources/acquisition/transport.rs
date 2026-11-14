//! The one way a request leaves the hub for a source (K-E1 Section 7).
//!
//! The URL is always a declared `api_base` plus the request's path; a path that could point
//! elsewhere, on the host or off it, is refused before anything is sent. The resolver resolves
//! only the host names of the declared API bases, and each address it returns must pass the
//! hub's address policy (R-030, P3-003), which allows private and VPN ranges: it is not what
//! keeps the token from another host. TLS name verification is: the server must prove it is
//! the declared host before the token is sent. The client follows no redirect,
//! takes no proxy from the environment, speaks HTTPS only, gives up after 20 seconds and reads
//! at most 8 MiB. The token header is marked sensitive, and no URL, header or body reaches a
//! log or an error (LOG-007).

use std::{error::Error as StdError, net::IpAddr, sync::Arc, time::Duration};

use reqwest::{
    Client, Response, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    header::{ACCEPT, AUTHORIZATION, HeaderValue},
    redirect::Policy,
};
use source_exchange::{Method, Request};
use zeroize::Zeroizing;

use super::{tokens::Tokens, verdict::Limits};
use crate::fleet::{
    credentials::Secret,
    targets::{AddressPolicy, resolve_checked},
};

/// The most body bytes read.
pub const BODY_CAP: usize = 8 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Why nothing usable came back. Fixed words: no URL, header or body (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("the request names a path or a host the hub does not send to")]
    Refused,
    #[error("the source could not be reached")]
    Network,
    #[error("the source's reply is larger than the hub reads")]
    TooLarge,
}

/// The transport could not be built. Fixed words.
#[derive(Debug, thiserror::Error)]
#[error("the source transport could not be built")]
pub struct BuildFailed(#[source] reqwest::Error);

/// What a source answered.
pub struct Answer {
    pub status: u16,
    pub limits: Limits,
    /// Wiped on drop: a mint reply holds a token.
    pub body: Zeroizing<Vec<u8>>,
}

/// The hub's client for sources, with the installation tokens it sends. Cheap to clone; clones
/// share the tokens.
#[derive(Clone)]
pub struct Transport {
    client: Client,
    /// The host names of the declared API bases: the only destinations.
    allowed: Arc<[String]>,
    /// Replaces every API base: a loopback stub, set only by tests.
    base_override: Option<Url>,
    tokens: Tokens,
}

/// Resolves only the allowed names, and only to addresses the hub's address policy permits
/// (private and VPN ranges included). TLS name verification, not this, ties the connection to
/// the declared host.
struct AllowedHosts {
    hosts: Arc<[String]>,
    policy: AddressPolicy,
}

#[derive(Debug, thiserror::Error)]
#[error("the host is not an allowed source")]
struct NotAllowed;

fn is_allowed(hosts: &[String], host: &str) -> bool {
    hosts
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(host))
}

impl Resolve for AllowedHosts {
    fn resolve(&self, name: Name) -> Resolving {
        let allowed = is_allowed(&self.hosts, name.as_str());
        let policy = self.policy.clone();
        Box::pin(async move {
            if !allowed {
                return Err(Box::new(NotAllowed) as Box<dyn StdError + Send + Sync>);
            }
            let addresses = resolve_checked(name.as_str(), 0, &policy)
                .await
                .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

fn client(
    hosts: Arc<[String]>,
    policy: AddressPolicy,
    https_only: bool,
) -> Result<Client, BuildFailed> {
    Client::builder()
        .use_rustls_tls()
        .user_agent("anima-hub")
        .no_proxy()
        .redirect(Policy::none())
        .https_only(https_only)
        .timeout(TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .dns_resolver(Arc::new(AllowedHosts { hosts, policy }))
        .build()
        .map_err(BuildFailed)
}

/// The host name of an `https` API base; none for an address, which would skip the resolver.
fn host_name(base: &str) -> Option<String> {
    let url = Url::parse(base)
        .ok()
        .filter(|url| url.scheme() == "https")?;
    let host = url.host_str()?;
    let literal = host.starts_with('[') || host.parse::<IpAddr>().is_ok();
    (!literal).then(|| host.to_ascii_lowercase())
}

impl Transport {
    /// The transport for the sources at `api_bases`, the declared bases: only their host
    /// names resolve, and only to addresses `policy` permits.
    ///
    /// # Errors
    /// [`BuildFailed`] when the TLS client cannot be built.
    pub fn new<'a>(
        api_bases: impl IntoIterator<Item = &'a str>,
        policy: AddressPolicy,
    ) -> Result<Self, BuildFailed> {
        let hosts: Arc<[String]> = api_bases.into_iter().filter_map(host_name).collect();
        Ok(Self {
            client: client(hosts.clone(), policy, true)?,
            allowed: hosts,
            base_override: None,
            tokens: Tokens::default(),
        })
    }

    /// A transport that sends every request to `stub` instead, over plain HTTP: for tests
    /// with a loopback stub source. `policy` must permit the stub's address.
    ///
    /// # Errors
    /// [`BuildFailed`] when the client cannot be built.
    #[cfg(any(test, feature = "test-support"))]
    pub fn to_stub(stub: Url, policy: AddressPolicy) -> Result<Self, BuildFailed> {
        let hosts: Arc<[String]> = stub.host_str().map(str::to_owned).into_iter().collect();
        Ok(Self {
            client: client(hosts.clone(), policy, false)?,
            allowed: hosts,
            base_override: Some(stub),
            tokens: Tokens::default(),
        })
    }

    /// The installation tokens this transport sends, in memory only (PRJ-018).
    #[must_use]
    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// The URL of `request` below `api_base`, or [`Fault::Refused`] when the base is not an
    /// allowed destination or the path could leave it.
    ///
    /// # Errors
    /// [`Fault::Refused`].
    pub fn url(&self, api_base: &str, request: &Request) -> Result<Url, Fault> {
        let base = match &self.base_override {
            Some(stub) => stub.clone(),
            None => Url::parse(api_base)
                .ok()
                .filter(|base| base.scheme() == "https")
                .ok_or(Fault::Refused)?,
        };
        let allowed = base
            .host_str()
            .is_some_and(|host| is_allowed(&self.allowed, host));
        if !allowed {
            return Err(Fault::Refused);
        }
        address(&base, &request.path, &request.query)
    }

    /// Sends the provider's `request` to `url`, its address, with the installation token.
    /// There is only `GET`: a provider cannot express a write.
    pub(in crate::engine::sources) async fn get(
        &self,
        request: &Request,
        url: Url,
        token: &Secret,
    ) -> Result<Answer, Fault> {
        let Method::Get = request.method;
        let sent = self
            .client
            .get(url)
            .header(AUTHORIZATION, bearer(token.expose())?)
            .header(ACCEPT, HeaderValue::from_static("application/json"));
        read(sent.send().await.map_err(|_| Fault::Network)?).await
    }

    /// Asks for an installation token at `url`, signed in with `app_token`.
    pub(in crate::engine::sources) async fn mint(
        &self,
        url: Url,
        app_token: &str,
    ) -> Result<Answer, Fault> {
        let sent = self
            .client
            .post(url)
            .header(AUTHORIZATION, bearer(app_token.as_bytes())?)
            .header(ACCEPT, HeaderValue::from_static("application/json"));
        read(sent.send().await.map_err(|_| Fault::Network)?).await
    }
}

/// `Bearer <token>`, marked sensitive so it never appears in a `Debug` output.
fn bearer(token: &[u8]) -> Result<HeaderValue, Fault> {
    let mut text = Zeroizing::new(Vec::with_capacity(token.len() + 7));
    text.extend_from_slice(b"Bearer ");
    text.extend_from_slice(token);
    let mut value = HeaderValue::from_bytes(&text).map_err(|_| Fault::Refused)?;
    value.set_sensitive(true);
    Ok(value)
}

/// Whether `path` stays below the base: one leading slash, no scheme, no user, no query or
/// fragment of its own, no backslash, no control or space character, no `.` or `..` segment
/// and no percent-encoded dot (the URL parser resolves both, so the token would go to another
/// path on the same host).
fn path_is_plain(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("://")
        && !path
            .chars()
            .any(|c| matches!(c, '@' | '?' | '#' | '\\') || c.is_control() || c.is_whitespace())
        && !path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        && !path
            .as_bytes()
            .windows(3)
            .any(|w| w[0] == b'%' && w[1] == b'2' && w[2].eq_ignore_ascii_case(&b'e'))
}

/// `base` with `path` appended and `query` set. Refused when the path could leave the base, or
/// when the result's scheme, host or port is not the base's, or its path is not below the
/// base's path.
///
/// # Errors
/// [`Fault::Refused`].
pub fn address(base: &Url, path: &str, query: &[(String, String)]) -> Result<Url, Fault> {
    if !path_is_plain(path) || !base.username().is_empty() || base.password().is_some() {
        return Err(Fault::Refused);
    }
    let prefix = base.path().trim_end_matches('/');
    let mut url = base.clone();
    url.set_path(&format!("{prefix}{path}"));
    url.set_query(None);
    url.set_fragment(None);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    let same = url.scheme() == base.scheme()
        && url.host_str() == base.host_str()
        && url.port_or_known_default() == base.port_or_known_default()
        && url
            .path()
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'));
    if same { Ok(url) } else { Err(Fault::Refused) }
}

/// The status, the rate-limit headers and at most [`BODY_CAP`] bytes of the body.
async fn read(mut response: Response) -> Result<Answer, Fault> {
    let status = response.status().as_u16();
    let limits = Limits::of(response.headers());
    let expected = response
        .content_length()
        .and_then(|n| usize::try_from(n).ok());
    let mut body = Zeroizing::new(Vec::with_capacity(expected.unwrap_or(0).min(BODY_CAP)));
    while let Some(chunk) = response.chunk().await.map_err(|_| Fault::Network)? {
        if body.len() + chunk.len() > BODY_CAP {
            return Err(Fault::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Answer {
        status,
        limits,
        body,
    })
}
