//! The read of one digest at a public registry (HOST-021): at most two HTTPS requests to
//! fixed hosts, each pinned to the checked addresses of its host.
//!
//! The rules are those of the RDAP path (`fleet/checks/probe/rdap/egress.rs`, P3-003): each
//! host is resolved once with `targets::resolve_checked`, which refuses it when the address
//! policy refuses any address it names; the client reaches the host only at those addresses
//! and resolves no name itself; TLS is verified against the built-in roots; no proxy, no
//! cookie, no compression. Here no redirect is followed at all, and no credential is sent:
//! the token is the one a registry gives to a reader with no account. The manifest is asked
//! for with `HEAD`, so no layer and no manifest body is read. A [`FetchError`] names no URL,
//! host, address or reply (LOG-007).

use std::{error::Error, future, sync::Arc, time::Duration};

use reqwest::{
    Certificate, Client, Method, StatusCode, Url,
    dns::{Name, Resolve, Resolving},
    header::{ACCEPT, AUTHORIZATION},
    redirect::Policy,
};
use serde::Deserialize;

use super::registry::Reference;
use crate::fleet::targets::{AddressPolicy, resolve_checked};

/// The longest one request may take.
const REQUEST_LIMIT: Duration = Duration::from_secs(10);
/// The most bytes of a token answer the hub reads.
const TOKEN_CAP: usize = 64 * 1024;
/// The manifest forms the hub accepts: an index of several platforms first, because the
/// digest that Docker keeps for a tag is the digest of the index.
const MANIFESTS: &str = "application/vnd.oci.image.index.v1+json, \
     application/vnd.docker.distribution.manifest.list.v2+json, \
     application/vnd.oci.image.manifest.v1+json, \
     application/vnd.docker.distribution.manifest.v2+json";
const DIGEST_HEADER: &str = "docker-content-digest";

/// Why a registry gave no digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(in crate::work::containers) enum FetchError {
    /// The registry has no such tag for a reader with no account: it is private, or gone.
    #[error("the registry has no such image for a reader with no account")]
    NotThere,
    /// A host does not resolve, names an address the hub may not reach, or did not answer
    /// as a registry does.
    #[error("the registry was not read")]
    Failed,
}

/// The rules a connection to a registry obeys.
#[derive(Clone, Debug)]
pub(in crate::work::containers) struct Egress {
    policy: AddressPolicy,
    /// DER root certificates trusted besides the built-in roots, and the one origin that
    /// stands for every registry host. Only tests set them.
    extra_roots: Vec<Vec<u8>>,
    stand_in: Option<Url>,
}

#[derive(Debug, thiserror::Error)]
#[error("the registry client resolves no names")]
struct LookupRefused;

/// A resolver that refuses every name, so a host is reached only at its pinned addresses.
struct NoLookup;

impl Resolve for NoLookup {
    fn resolve(&self, _name: Name) -> Resolving {
        Box::pin(future::ready(Err(
            Box::new(LookupRefused) as Box<dyn Error + Send + Sync>
        )))
    }
}

#[derive(Deserialize)]
struct TokenAnswer {
    token: String,
}

impl Egress {
    pub(in crate::work::containers) const fn new(policy: AddressPolicy) -> Self {
        Self {
            policy,
            extra_roots: Vec::new(),
            stand_in: None,
        }
    }

    /// For tests: every registry host is the local server at `origin`, whose certificate
    /// `der` is trusted.
    #[cfg(test)]
    #[must_use]
    pub(in crate::work::containers) fn standing_in(mut self, origin: Url, der: Vec<u8>) -> Self {
        self.extra_roots.push(der);
        self.stand_in = Some(origin);
        self
    }

    /// `https://host/path`, with a path of static text and checked characters only.
    fn url(&self, host: &str, path: &str) -> Result<Url, FetchError> {
        let origin = match &self.stand_in {
            Some(origin) => origin.clone(),
            None => Url::parse(&format!("https://{host}/")).map_err(|_| FetchError::Failed)?,
        };
        origin.join(path).map_err(|_| FetchError::Failed)
    }

    /// Where the manifest of `reference` is.
    pub(in crate::work::containers) fn manifest_url(
        &self,
        reference: &Reference,
    ) -> Result<Url, FetchError> {
        let path = format!("/v2/{}/manifests/{}", reference.repository, reference.tag);
        self.url(reference.registry.host(), &path)
    }

    /// Where `reference` gets its token; `None` for a registry that asks for none.
    pub(in crate::work::containers) fn token_url(
        &self,
        reference: &Reference,
    ) -> Result<Option<Url>, FetchError> {
        let Some((host, path, service)) = reference.registry.token() else {
            return Ok(None);
        };
        let scope = format!("repository:{}:pull", reference.repository);
        let mut url = self.url(host, path)?;
        url.query_pairs_mut()
            .append_pair("service", service)
            .append_pair("scope", &scope);
        Ok(Some(url))
    }

    /// The digest the registry gives now for the tag of `reference`.
    ///
    /// # Errors
    /// [`FetchError::NotThere`] when the registry refuses a reader with no account or knows
    /// no such tag; [`FetchError::Failed`] for every other reason.
    pub(in crate::work::containers) async fn digest(
        &self,
        reference: &Reference,
    ) -> Result<String, FetchError> {
        let token = match self.token_url(reference)? {
            Some(url) => Some(self.token(url).await?),
            None => None,
        };
        let url = self.manifest_url(reference)?;
        let mut request = self
            .pinned_client(&url)
            .await?
            .request(Method::HEAD, url)
            .header(ACCEPT, MANIFESTS);
        if let Some(token) = token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = request.send().await.map_err(|_| FetchError::Failed)?;
        match response.status() {
            StatusCode::OK => response
                .headers()
                .get(DIGEST_HEADER)
                .and_then(|value| value.to_str().ok())
                .filter(|digest| is_digest(digest))
                .map(str::to_owned)
                .ok_or(FetchError::Failed),
            StatusCode::NOT_FOUND | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(FetchError::NotThere)
            }
            _ => Err(FetchError::Failed),
        }
    }

    /// The token a registry gives to a reader with no account.
    async fn token(&self, url: Url) -> Result<String, FetchError> {
        let client = self.pinned_client(&url).await?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|_| FetchError::Failed)?;
        // A token endpoint that refuses says nothing of the image: the registry was not read.
        if response.status() != StatusCode::OK {
            return Err(FetchError::Failed);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| FetchError::Failed)? {
            if chunk.len() > TOKEN_CAP - body.len() {
                return Err(FetchError::Failed);
            }
            body.extend_from_slice(&chunk);
        }
        let answer: TokenAnswer = serde_json::from_slice(&body).map_err(|_| FetchError::Failed)?;
        let plain =
            !answer.token.is_empty() && answer.token.bytes().all(|byte| byte.is_ascii_graphic());
        plain.then_some(answer.token).ok_or(FetchError::Failed)
    }

    /// A client that reaches the host of `url` only at its checked addresses, follows no
    /// redirect and resolves no other name.
    async fn pinned_client(&self, url: &Url) -> Result<Client, FetchError> {
        let host = url.host_str().ok_or(FetchError::Failed)?;
        let port = url.port_or_known_default().ok_or(FetchError::Failed)?;
        let addresses = resolve_checked(host, port, &self.policy)
            .await
            .map_err(|_| FetchError::Failed)?;
        let mut builder = Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .https_only(true)
            .user_agent("anima-hub")
            .no_proxy()
            .redirect(Policy::none())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .timeout(REQUEST_LIMIT)
            .dns_resolver(Arc::new(NoLookup));
        if let Some(domain) = url.domain() {
            builder = builder.resolve_to_addrs(domain, &addresses);
        }
        for der in &self.extra_roots {
            let root = Certificate::from_der(der).map_err(|_| FetchError::Failed)?;
            builder = builder.add_root_certificate(root);
        }
        builder.build().map_err(|_| FetchError::Failed)
    }
}

/// True for `sha256:` and 64 lower-case hex digits.
pub(in crate::work::containers) fn is_digest(text: &str) -> bool {
    text.strip_prefix("sha256:").is_some_and(|digits| {
        digits.len() == 64
            && digits
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
