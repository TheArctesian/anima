//! HTTP egress: one POST over a client pinned to the checked addresses of one host.
//!
//! The client resolves no name itself, follows no redirect, uses no proxy, keeps no cookie,
//! decompresses nothing and verifies TLS with the built-in roots. It reads at most
//! [`REPLY_CAP`] bytes of the reply body and discards them.

use std::{error::Error, future, net::SocketAddr, sync::Arc};

use reqwest::{
    Certificate, Client, Response, Url,
    dns::{Name, Resolve, Resolving},
    header::HeaderMap,
    redirect::Policy,
};

use super::{Egress, SendError, TIME_LIMIT, causes, tls_broke};
use crate::targets::{TargetError, resolve_checked};

/// The most reply body bytes read; the rest is never read (D-17).
pub(super) const REPLY_CAP: usize = 4 * 1024;

/// What the client's resolver answers for every name: the client looks up nothing, so a host
/// is reached only at the addresses pinned for it.
#[derive(Debug, thiserror::Error)]
#[error("the notification client resolves no names")]
struct LookupRefused;

/// A resolver that refuses every name.
struct NoLookup;

impl Resolve for NoLookup {
    fn resolve(&self, _name: Name) -> Resolving {
        Box::pin(future::ready(Err(
            Box::new(LookupRefused) as Box<dyn Error + Send + Sync>
        )))
    }
}

impl Egress {
    /// Sends `body` with `headers` to `url` as one POST. The host is resolved once and every
    /// address checked; the request goes only to those addresses. Every header value is
    /// marked sensitive, so none is printed in a debug view.
    ///
    /// # Errors
    /// [`SendError::Url`] for a URL that is not http or https; [`SendError::Target`] when the
    /// host does not resolve or names a refused address, with no connection made;
    /// [`SendError::HttpStatus`] for an answer outside 2xx (a redirect included); otherwise
    /// the connection, TLS or time limit failure.
    pub(crate) async fn post(
        &self,
        url: &Url,
        mut headers: HeaderMap,
        body: Vec<u8>,
    ) -> Result<(), SendError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(SendError::Url);
        }
        let host = url.host_str().ok_or(SendError::Url)?;
        let port = url.port_or_known_default().ok_or(SendError::Url)?;
        let addresses = resolve_checked(host, port, &self.policy).await?;
        let client = self.pinned_client(url.domain(), &addresses)?;
        for value in headers.values_mut() {
            value.set_sensitive(true);
        }
        let response = client
            .post(url.clone())
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|error| classify(&error))?;
        judge(response).await
    }

    /// A client that reaches `domain` only at `addresses` and resolves no other name. An
    /// IP-literal host (`domain` is `None`) is reached at the address it names.
    pub(super) fn pinned_client(
        &self,
        domain: Option<&str>,
        addresses: &[SocketAddr],
    ) -> Result<Client, SendError> {
        let mut builder = Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .user_agent("anima-hub")
            .no_proxy()
            .redirect(Policy::none())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .timeout(TIME_LIMIT)
            .dns_resolver(Arc::new(NoLookup));
        if let Some(domain) = domain {
            builder = builder.resolve_to_addrs(domain, addresses);
        }
        for der in &self.extra_roots {
            let root = Certificate::from_der(der).map_err(|_| SendError::Tls)?;
            builder = builder.add_root_certificate(root);
        }
        builder.build().map_err(|_| SendError::Tls)
    }
}

/// Success for a 2xx answer, after reading and discarding at most [`REPLY_CAP`] body bytes.
async fn judge(mut response: Response) -> Result<(), SendError> {
    let status = response.status();
    if !status.is_success() {
        return Err(SendError::HttpStatus(status.as_u16()));
    }
    discard_reply(&mut response).await;
    Ok(())
}

/// Reads and drops body chunks until [`REPLY_CAP`] bytes are read or the body ends, and
/// returns how many were read. The status has already decided the outcome, so a body that
/// breaks off or runs past the time limit ends the read and changes nothing.
pub(super) async fn discard_reply(response: &mut Response) -> usize {
    let mut read = 0;
    while read < REPLY_CAP {
        match response.chunk().await {
            Ok(Some(chunk)) => read += chunk.len(),
            Ok(None) | Err(_) => break,
        }
    }
    read
}

/// The [`SendError`] a reqwest error stands for, judged from its causes, never its text (the
/// text holds the URL).
pub(super) fn classify(error: &reqwest::Error) -> SendError {
    if causes(error).any(<dyn Error>::is::<LookupRefused>) {
        SendError::Target(TargetError::Unresolved)
    } else if error.is_timeout() {
        SendError::TimedOut
    } else if tls_broke(error) {
        SendError::Tls
    } else if error.is_connect() {
        SendError::Connect
    } else {
        SendError::Exchange
    }
}
