//! The agent's one HTTP client and its three calls to the hub (K8 Section 1).
//!
//! The client verifies certificates against the web PKI roots, follows no redirect, ignores
//! proxy variables, keeps no cookie, and gives up after 10 seconds (MET-020). It logs nothing:
//! its callers log at their own boundary, and never a code, a token or a body (LOG-007).

pub mod answer;
pub mod enrolment;

use std::time::Duration;

use agent_core::wire::{
    AgentToken, Batch, EnrolRequest, Enrolled, EnrolmentCode, RenewRequest, Renewed,
};
use reqwest::{Client, Url, redirect};

use answer::{Answer, Cause, answer};

/// How long one request may take, connection and answer together (K8 Section 7).
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The most of an answer the agent reads: far above any K8 answer.
const ANSWER_LIMIT: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("the HTTP client could not be built")]
pub struct HubError;

/// One request: its route, its bearer (none for enrolment), its JSON body and the status that
/// means success.
struct Call<'a> {
    route: &'static str,
    bearer: Option<&'a AgentToken>,
    body: Vec<u8>,
    ok_status: u16,
}

/// A client for one hub.
pub struct HubClient {
    http: Client,
    base: Url,
}

impl HubClient {
    /// A client for the hub at `base`. Pass an [`Origin`](crate::config::origin::Origin)'s URL; tests
    /// pass a local `http` URL, since the https rule belongs to the origin's parser.
    ///
    /// # Errors
    /// [`HubError`] when the TLS roots cannot be set up.
    pub fn new(base: Url) -> Result<Self, HubError> {
        let http = Client::builder()
            .redirect(redirect::Policy::none())
            .no_proxy()
            .timeout(TIMEOUT)
            .build()
            .map_err(|_| HubError)?;
        Ok(Self { http, base })
    }

    /// `POST /agent/v1/enrol` (K8 Section 3). The code is spent by this call.
    pub async fn enrol(&self, code: EnrolmentCode) -> Answer<Enrolled> {
        let call = Call {
            route: "agent/v1/enrol",
            bearer: None,
            body: json(&EnrolRequest { code }),
            ok_status: 200,
        };
        self.post(call, read_json).await
    }

    /// `POST /agent/v1/push` (K8 Section 4).
    pub async fn push(&self, token: &AgentToken, batch: &Batch) -> Answer<()> {
        let call = Call {
            route: "agent/v1/push",
            bearer: Some(token),
            body: json(batch),
            ok_status: 204,
        };
        self.post(call, |_| Some(())).await
    }

    /// `POST /agent/v1/renew` with the current token (K8 Section 5).
    pub async fn renew(&self, token: &AgentToken) -> Answer<Renewed> {
        let call = Call {
            route: "agent/v1/renew",
            bearer: Some(token),
            body: json(&RenewRequest {}),
            ok_status: 200,
        };
        self.post(call, read_json).await
    }

    async fn post<T>(&self, call: Call<'_>, read_ok: impl FnOnce(&[u8]) -> Option<T>) -> Answer<T> {
        let Ok(url) = self.base.join(call.route) else {
            return Answer::Failed(Cause::Unreadable);
        };
        let mut request = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .body(call.body);
        if let Some(token) = call.bearer {
            request = request.bearer_auth(token.expose());
        }
        let mut response = match request.send().await {
            Ok(response) => response,
            Err(error) => return Answer::Failed(cause(&error)),
        };
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        while bytes.len() < ANSWER_LIMIT {
            match response.chunk().await {
                Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(error) => return Answer::Failed(cause(&error)),
            }
        }
        answer(status, &bytes, call.ok_status, read_ok)
    }
}

fn cause(error: &reqwest::Error) -> Cause {
    if error.is_timeout() {
        Cause::Timeout
    } else {
        Cause::Network
    }
}

fn json(value: &impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(value).expect("a K8 request body serialises")
}

fn read_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    serde_json::from_slice(bytes).ok()
}
