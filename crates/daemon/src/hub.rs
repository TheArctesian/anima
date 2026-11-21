//! The daemon's one HTTP client and its calls to the hub (K8 Section 1; K-E5 Section 2.1).
//!
//! In the link mode the client sends plain HTTP on the Unix socket of `anima-link`, and to no
//! TCP port (R-172). It writes a request only to a connection whose peer passed its check
//! ([`peer`]).
//!
//! The client verifies certificates against the web PKI roots, follows no redirect, ignores
//! proxy variables, keeps no cookie, and gives up after 10 seconds (MET-020). It logs nothing:
//! its callers log at their own boundary, and never a code, a token or a body (LOG-007).

pub mod answer;
pub mod body;
pub mod enrolment;
pub mod link_socket;
#[cfg(unix)]
pub mod peer;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::path::Path;
use std::time::Duration;

use bytes::Bytes;
use daemon_core::wire::{
    Batch, DaemonToken, EnrolRequest, Enrolled, EnrolmentCode, FetchedKey, KeyFetch, PushAccepted,
    RenewRequest, Renewed,
};
use reqwest::{Client, Url, redirect};
use zeroize::Zeroizing;

use crate::config::origin::Reach;
use answer::{Answer, Cause, answer};
use body::Body;
use link_socket::LinkSocketError;
#[cfg(unix)]
use peer::{Expected, PeerError};
#[cfg(unix)]
use unix::Local;
#[cfg(windows)]
use windows::Local;

/// How long one request may take, connection and answer together (K8 Section 7).
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The base URL of a client for the socket of the link. No request goes to this address: the
/// client connects to the socket and sets the loopback `Host` itself.
const ON_LINK_SOCKET: &str = "http://127.0.0.1/";

#[derive(Debug, thiserror::Error)]
pub enum HubError {
    #[error("the HTTP client could not be built")]
    Build,
    #[error(transparent)]
    LinkSocket(#[from] LinkSocketError),
    #[cfg(unix)]
    #[error(transparent)]
    Peer(#[from] PeerError),
    /// On macOS the token is in the Keychain, and no value of the environment may decide where
    /// it goes: the folder of an old link origin does (the finding `daemonoldlinkhome` of the review HM-T23).
    #[error(
        "this daemon was enrolled with an old link origin, which macOS no longer uses: enrol it again (sign in again in the Anima application, or run anima-daemon enrol)"
    )]
    OldLinkOrigin,
}

/// One request: its route, its bearer (none for enrolment), its JSON body and the statuses that
/// mean success. A push takes `204`, and `200` with `Accepted` (K8 Section 8.2).
struct Call<'a> {
    route: &'static str,
    bearer: Option<&'a DaemonToken>,
    body: Zeroizing<Vec<u8>>,
    ok: &'static [u16],
}

/// A client for one hub.
pub struct HubClient {
    http: Client,
    base: Url,
    local: Option<Local>,
}

impl HubClient {
    /// A client for the hub at `base` that gives up on a request after `limit`, connection and
    /// answer together (callers pass [`TIMEOUT`]). Pass an [`Origin`](crate::config::origin::Origin)'s
    /// URL; the rule for what a hub origin is belongs to the origin's parser.
    ///
    /// # Errors
    /// [`HubError`] when the TLS roots cannot be set up.
    pub fn new(base: Url, limit: Duration) -> Result<Self, HubError> {
        let http = Client::builder()
            .redirect(redirect::Policy::none())
            .no_proxy()
            .timeout(limit)
            .build()
            .map_err(|_| HubError::Build)?;
        Ok(Self {
            http,
            base,
            local: None,
        })
    }

    /// A client for the hub behind the socket of `anima-link` at `socket` (K8 Section 1). Each
    /// request connects to the socket and to nothing else. On Unix it goes only to the peer that
    /// this daemon expects ([`Expected::of_this_daemon`]); Windows checks the access list of
    /// the socket.
    ///
    /// # Errors
    /// [`HubError::LinkSocket`] when a different user could listen at the path;
    /// [`HubError::Peer`] when the user or the signature of this daemon cannot be read.
    pub fn over_link(socket: &Path, limit: Duration) -> Result<Self, HubError> {
        #[cfg(unix)]
        return Self::over_link_to(socket, limit, Expected::of_this_daemon()?);
        #[cfg(windows)]
        {
            link_socket::trusted(socket)?;
            let mut client = Self::on_link_socket(limit)?;
            client.local = Some(Local::new(socket.to_path_buf(), limit));
            Ok(client)
        }
    }

    /// As [`HubClient::over_link`], with the peer that a request may go to given as `expected`.
    ///
    /// # Errors
    /// [`HubError::LinkSocket`] when a different user could listen at the path.
    #[cfg(unix)]
    pub fn over_link_to(
        socket: &Path,
        limit: Duration,
        expected: Expected,
    ) -> Result<Self, HubError> {
        link_socket::trusted(socket)?;
        let mut client = Self::on_link_socket(limit)?;
        client.local = Some(Local::new(socket.to_path_buf(), limit, expected));
        Ok(client)
    }

    fn on_link_socket(limit: Duration) -> Result<Self, HubError> {
        Self::new(
            Url::parse(ON_LINK_SOCKET).map_err(|_| HubError::Build)?,
            limit,
        )
    }

    /// A client for the hub that `reach` names; `own_socket` is the socket in the daemon's own
    /// folder, for a daemon that was enrolled with an old link origin.
    ///
    /// # Errors
    /// [`HubError`] as [`HubClient::new`] and [`HubClient::over_link`].
    pub fn reaching(
        reach: Reach<'_>,
        own_socket: &Path,
        limit: Duration,
    ) -> Result<Self, HubError> {
        match reach {
            Reach::Https(url) => Self::new(url.clone(), limit),
            Reach::LinkSocket(socket) => Self::over_link(socket, limit),
            Reach::OwnLinkSocket if cfg!(target_os = "macos") => Err(HubError::OldLinkOrigin),
            Reach::OwnLinkSocket => Self::over_link(own_socket, limit),
        }
    }

    /// `POST /daemon/v1/enrol` (K8 Section 3). The code is spent by this call.
    pub async fn enrol(&self, code: EnrolmentCode) -> Answer<Enrolled> {
        let call = Call {
            route: "daemon/v1/enrol",
            bearer: None,
            body: json(&EnrolRequest { code }),
            ok: &[200],
        };
        self.post(call, |_, body| read_json(body)).await
    }

    /// `POST /daemon/v1/push` (K8 Section 4): `None` for `204`, the `Accepted` of a `200`.
    pub async fn push(&self, token: &DaemonToken, batch: &Batch) -> Answer<Option<PushAccepted>> {
        let call = Call {
            route: "daemon/v1/push",
            bearer: Some(token),
            body: json(batch),
            ok: &[204, 200],
        };
        // An `Accepted` that does not read is ignored; the batch is stored either way.
        self.post(call, |status, body| {
            Some((status == 200).then(|| read_json(body)).flatten())
        })
        .await
    }

    /// `POST /daemon/v1/keys/fetch` (K-E5 Section 2.1).
    pub async fn fetch_key(&self, token: &DaemonToken, asked: &KeyFetch) -> Answer<FetchedKey> {
        let call = Call {
            route: "daemon/v1/keys/fetch",
            bearer: Some(token),
            body: json(asked),
            ok: &[200],
        };
        self.post(call, |_, body| read_json(body)).await
    }

    /// `POST /daemon/v1/renew` with the current token (K8 Section 5).
    pub async fn renew(&self, token: &DaemonToken) -> Answer<Renewed> {
        let call = Call {
            route: "daemon/v1/renew",
            bearer: Some(token),
            body: json(&RenewRequest {}),
            ok: &[200],
        };
        self.post(call, |_, body| read_json(body)).await
    }

    async fn post<T>(
        &self,
        call: Call<'_>,
        read_ok: impl FnOnce(u16, &[u8]) -> Option<T>,
    ) -> Answer<T> {
        if let Some(local) = &self.local {
            return local.post(call, read_ok).await;
        }
        let Ok(url) = self.base.join(call.route) else {
            return Answer::Failed(Cause::Unreadable);
        };
        let mut request = self
            .http
            .post(url)
            .header("content-type", "application/json")
            // The request owns the body as `Bytes`, so the one copy is wiped when the last
            // reference to it drops, and no other copy is made by the daemon. (hyper and rustls
            // copy the request body into their own buffers, which are not wiped: a known
            // remainder, as for the answer in `body`.)
            .body(Bytes::from_owner(call.body));
        if let Some(token) = call.bearer {
            request = request.bearer_auth(token.expose());
        }
        let mut response = match request.send().await {
            Ok(response) => response,
            Err(error) => return Answer::Failed(cause(&error)),
        };
        let status = response.status().as_u16();
        let mut bytes = Body::new();
        while !bytes.is_full() {
            match response.chunk().await {
                Ok(Some(chunk)) => bytes.add(&chunk),
                Ok(None) => break,
                Err(error) => return Answer::Failed(cause(&error)),
            }
        }
        if call.ok.contains(&status) {
            let read = read_ok(status, bytes.bytes());
            return read.map_or(Answer::Failed(Cause::Unreadable), Answer::Accepted);
        }
        // Not a success status, so the reader of a success is never called.
        answer(status, bytes.bytes(), call.ok[0], |_| None)
    }
}

fn cause(error: &reqwest::Error) -> Cause {
    if error.is_timeout() {
        Cause::Timeout
    } else {
        Cause::Network
    }
}

fn json(value: &impl serde::Serialize) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(serde_json::to_vec(value).expect("a K8 request body serialises"))
}

fn read_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    serde_json::from_slice(bytes).ok()
}
