//! The hub's own VPN status, read through the `LocalAPI` proxy (VPN-008).
//!
//! The proxy is operator infrastructure, as for the Tailscale ping check
//! (`checks/probe/tailscale.rs`). The reply is never logged or stored; errors hold fixed words
//! only (LOG-007).

use std::time::Duration;

use reqwest::{Client, Response, StatusCode, redirect::Policy};

use super::{
    Error,
    peers::{Status, parse},
};

/// The most reply bytes read: the proxy's own cap (1 MiB).
const REPLY_CAP: usize = 1024 * 1024;
/// No caller cuts this request off, so it has its own limit.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The status of the hub's node and its peers.
///
/// # Errors
/// `Request` when the proxy does not answer in time, `Refused` on 403, `Status` on another
/// code, `TooLong` past 1 MiB, `Unreadable` on a reply of another shape.
pub(super) async fn status(localapi: &str) -> Result<Status, Error> {
    let url = format!("http://{localapi}/localapi/v0/status?peers=true");
    let mut response = client()?
        .get(url)
        .send()
        .await
        .map_err(|_| Error::Request)?;
    match response.status() {
        StatusCode::OK => parse(&read_capped(&mut response).await?),
        StatusCode::FORBIDDEN => Err(Error::Refused),
        other => Err(Error::Status(other.as_u16())),
    }
}

fn client() -> Result<Client, Error> {
    Client::builder()
        .user_agent("anima-hub")
        .no_proxy()
        .redirect(Policy::none())
        .timeout(TIMEOUT)
        .build()
        .map_err(|_| Error::Request)
}

/// Reads at most [`REPLY_CAP`] bytes; one more byte than that is `TooLong`.
async fn read_capped(response: &mut Response) -> Result<Vec<u8>, Error> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Request)? {
        body.extend_from_slice(&chunk);
        if body.len() > REPLY_CAP {
            return Err(Error::TooLong);
        }
    }
    Ok(body)
}
