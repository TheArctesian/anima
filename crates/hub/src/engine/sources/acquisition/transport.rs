//! The one way a request leaves the hub for a source (K-E1 Section 7).
//!
//! The URL is always a declared `api_base` plus the request's path; a path that could point
//! elsewhere, on the host or off it, is refused before anything is sent. The resolver resolves
//! only the host names of the declared API bases, and each address it returns must pass the
//! hub's address policy (R-030, P3-003), which allows private and VPN ranges: it is not what
//! keeps the token from another host. TLS name verification is: the server must prove it is
//! the declared host before the token is sent. The client follows no redirect,
//! takes no proxy from the environment, speaks HTTPS only, gives up after 20 seconds and reads
//! at most 8 MiB: past that it stops reading, keeps none of the body and says so. The token header is marked sensitive, and no URL, header or body reaches a
//! log or an error (LOG-007).
//!
//! Every request names the API version it was written for and GitHub's media type (decision 1
//! of the Projects slice). A named change is the only request with a body the hub builds from a
//! provider: the transport renders it itself from a checked [`Change`], so no caller hands it a
//! path, a verb or a URL.

mod address;

use std::{error::Error as StdError, net::IpAddr, sync::Arc, time::Duration};

use reqwest::{
    Client, Response, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use source_exchange::{Change, Declaration, Limit, Method, Request, Verb};
use zeroize::Zeroizing;

pub use self::address::address;
use super::{tokens::Tokens, verdict::Limits};
use crate::{
    engine::sources::live::Live,
    fleet::{
        credentials::Secret,
        targets::{AddressPolicy, resolve_checked},
    },
};

/// The most body bytes read.
pub const BODY_CAP: usize = 8 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// The most bytes of a change's JSON body: a 65,536-character text, escaped, fits.
pub const CHANGE_BODY_CAP: usize = 1024 * 1024;
/// The REST API version every request is written for (K-E1 Section 13.1).
const API_VERSION: &str = "2022-11-28";
/// GitHub's JSON media type.
const MEDIA_TYPE: &str = "application/vnd.github+json";

/// Why nothing usable came back. Fixed words: no URL, header or body (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("the request names a path or a host the hub does not send to")]
    Refused,
    #[error("the source could not be reached")]
    Network,
}

/// The transport could not be built. Fixed words.
#[derive(Debug, thiserror::Error)]
#[error("the source transport could not be built")]
pub struct BuildFailed(#[source] reqwest::Error);

/// What a source answered.
pub struct Answer {
    pub status: u16,
    pub limits: Limits,
    pub content: Content,
    pub link: Option<String>,
}

impl Answer {
    /// The rate-limit headers as the provider's `settle` reads them: numbers, never text.
    #[must_use]
    pub fn limit(&self) -> Limit {
        Limit {
            remaining: self.limits.remaining,
            reset_at: self
                .limits
                .reset
                .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0)),
            retry_after_seconds: self.limits.retry_after,
        }
    }
}

/// What came of sending a change.
pub enum Delivery {
    Answered(Answer),
    /// The request may have reached the source, but no whole answer came back: whether the
    /// change was made is unknown. Never sent again.
    Lost,
}

/// The body of an answer, or the fact that it was larger than the hub reads.
pub enum Content {
    /// Wiped on drop: a mint reply holds a token, a conversion reply a key.
    Body(Zeroizing<Vec<u8>>),
    /// Past [`BODY_CAP`]: reading stopped there and what was read is wiped.
    TooLarge,
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
    /// The live path's cache, budget and pace (K-E1 Section 13.1).
    live: Live,
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
    let version = HeaderValue::from_static(API_VERSION);
    let headers =
        HeaderMap::from_iter([(HeaderName::from_static("x-github-api-version"), version)]);
    Client::builder()
        .use_rustls_tls()
        .user_agent("anima-hub")
        .default_headers(headers)
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
            live: Live::default(),
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
            live: Live::default(),
        })
    }

    /// The installation tokens this transport sends, in memory only (PRJ-018).
    #[must_use]
    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// The live path's cache, budget and pace, shared by clones.
    pub(in crate::engine::sources) const fn live(&self) -> &Live {
        &self.live
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
            .header(ACCEPT, HeaderValue::from_static(MEDIA_TYPE));
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
            .header(ACCEPT, HeaderValue::from_static(MEDIA_TYPE));
        read(sent.send().await.map_err(|_| Fault::Network)?).await
    }

    /// Converts a manifest code at `url`: a POST with no `Authorization` header and no body,
    /// because the one-time code in the path is the only proof the source asks for (PRJ-039).
    pub(in crate::engine::sources) async fn convert(&self, url: Url) -> Result<Answer, Fault> {
        let sent = self
            .client
            .post(url)
            .header(ACCEPT, HeaderValue::from_static("application/json"));
        read(sent.send().await.map_err(|_| Fault::Network)?).await
    }
}

impl Transport {
    /// Sends `change` to the source of `declaration` with the installation token: the
    /// provider's `render` gives the path below `api_base` and the JSON body, and the change
    /// fixes the verb. Refused before sending when the source makes no changes, the path fails
    /// the checks of [`Self::url`] or the body passes [`CHANGE_BODY_CAP`]; [`Fault::Network`]
    /// when the connection failed before anything was sent; [`Delivery::Lost`] once it may have
    /// been.
    pub(in crate::engine::sources) async fn change(
        &self,
        declaration: &Declaration,
        change: &Change,
        token: &Secret,
    ) -> Result<Delivery, Fault> {
        let changes = declaration.changes.ok_or(Fault::Refused)?;
        let rendered = (changes.render)(change);
        let target = Request::get(rendered.path(), &[]).map_err(|_| Fault::Refused)?;
        let url = self.url(declaration.api_base, &target)?;
        let body = match rendered.body() {
            Some(json) => Some(serde_json::to_vec(json).map_err(|_| Fault::Refused)?),
            None => None,
        };
        if body
            .as_ref()
            .is_some_and(|bytes| bytes.len() > CHANGE_BODY_CAP)
        {
            return Err(Fault::Refused);
        }
        let method = match rendered.verb() {
            Verb::Post => reqwest::Method::POST,
            Verb::Put => reqwest::Method::PUT,
            Verb::Patch => reqwest::Method::PATCH,
        };
        let mut sent = self
            .client
            .request(method, url)
            .header(AUTHORIZATION, bearer(token.expose())?)
            .header(ACCEPT, HeaderValue::from_static(MEDIA_TYPE));
        if let Some(bytes) = body {
            sent = sent
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .body(bytes);
        }
        match sent.send().await {
            Ok(response) => Ok(read(response)
                .await
                .map_or(Delivery::Lost, Delivery::Answered)),
            Err(error) if error.is_connect() => Err(Fault::Network),
            Err(_) => Ok(Delivery::Lost),
        }
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

/// The status, the rate-limit headers and the body, or [`Content::TooLarge`] as soon as the
/// declared length or the bytes read pass [`BODY_CAP`]: the rest is never read.
async fn read(mut response: Response) -> Result<Answer, Fault> {
    let status = response.status().as_u16();
    let limits = Limits::of(response.headers());
    let link = response
        .headers()
        .get(reqwest::header::LINK)
        .and_then(|header| header.to_str().ok())
        .map(str::to_owned);
    let content = content(&mut response).await?;
    Ok(Answer {
        status,
        limits,
        content,
        link,
    })
}

async fn content(response: &mut Response) -> Result<Content, Fault> {
    let expected = response
        .content_length()
        .and_then(|n| usize::try_from(n).ok());
    if expected.is_some_and(|n| n > BODY_CAP) {
        return Ok(Content::TooLarge);
    }
    let mut body = Zeroizing::new(Vec::with_capacity(expected.unwrap_or(0)));
    while let Some(chunk) = response.chunk().await.map_err(|_| Fault::Network)? {
        if body.len() + chunk.len() > BODY_CAP {
            return Ok(Content::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Content::Body(body))
}
