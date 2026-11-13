//! The only code that talks to the hub (CMD-003): `POST /graphql` with the bearer token over
//! rustls (K7). Commands and, later, the MCP tools call [`Hub::run`] with a typed document.
//!
//! The client follows no redirect, ignores proxy variables, keeps no cookie and gives up after
//! [`TIMEOUT`]. It logs nothing; no error it returns holds the token or the body (CMD-014).

pub mod answer;
pub mod error;
pub mod origin;
pub mod scalars;
pub mod token;

use std::time::Duration;

use bytes::Bytes;
use graphql_client::{GraphQLQuery, QueryBody};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use reqwest::{Client, redirect};
use serde::Serialize;
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

pub use error::{Code, FieldError, HubError};
pub use origin::Origin;
pub use token::{Token, TokenKind};

/// How long one request may take, connection and answer together.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The largest answer read; a page of 200 devices is well under it (the hub's page limit).
const MAX_ANSWER: usize = 32 * 1024 * 1024;

/// One hub and the token sent to it. `Debug` shows the origin and the token's kind only.
#[derive(Debug)]
pub struct Hub {
    origin: Origin,
    token: Option<Token>,
    http: Client,
}

impl Hub {
    /// A client for `origin` that sends `token`, or none (only `login` works without one).
    ///
    /// # Errors
    /// [`HubError::Setup`] when the TLS roots cannot be loaded.
    pub fn new(origin: Origin, token: Option<Token>) -> Result<Self, HubError> {
        let http = Client::builder()
            .redirect(redirect::Policy::none())
            .no_proxy()
            .timeout(TIMEOUT)
            .build()
            .map_err(|_| HubError::Setup)?;
        Ok(Self {
            origin,
            token,
            http,
        })
    }

    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    #[must_use]
    pub const fn token(&self) -> Option<&Token> {
        self.token.as_ref()
    }

    /// The token this client sent, to store it once the hub has accepted it.
    #[must_use]
    pub fn into_token(self) -> Option<Token> {
        self.token
    }

    /// Runs document `Q` with `variables` and returns its typed `data`.
    ///
    /// # Errors
    /// [`HubError`] for a refusal (with its K7 code) or a failed request.
    pub async fn run<Q: GraphQLQuery>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData, HubError> {
        self.send(&Q::build_query(variables)).await
    }

    /// Sends a request body that the caller built, so it can wipe secrets in the variables
    /// after the call (as `auth login` does with the password).
    ///
    /// # Errors
    /// As [`Hub::run`].
    pub async fn send<V: Serialize, R: DeserializeOwned>(
        &self,
        body: &QueryBody<V>,
    ) -> Result<R, HubError> {
        let (status, answer) = self.post(encode(body)).await?;
        answer::read(status, &answer)
    }

    /// Sends a document the operator wrote (`anima api`) and returns the whole answer.
    ///
    /// # Errors
    /// [`HubError`] when no JSON answer came; the answer's first GraphQL error is returned
    /// beside it, not as an error.
    pub async fn send_raw(
        &self,
        body: &serde_json::Value,
    ) -> Result<(serde_json::Value, Option<HubError>), HubError> {
        let (status, answer) = self.post(encode(body)).await?;
        answer::read_raw(status, &answer)
    }

    async fn post(&self, body: Zeroizing<Vec<u8>>) -> Result<(u16, Zeroizing<Vec<u8>>), HubError> {
        let mut request = self
            .http
            .post(self.origin.graphql())
            .header(CONTENT_TYPE, "application/json")
            .header(
                ACCEPT,
                "application/graphql-response+json, application/json",
            )
            .body(Bytes::from_owner(body));
        if let Some(token) = &self.token {
            request = request.header(AUTHORIZATION, bearer(token)?);
        }
        let mut response = request
            .send()
            .await
            .map_err(|error| self.unreachable(&error))?;
        let status = response.status().as_u16();
        let mut answer = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| self.unreachable(&error))?
        {
            if answer.len() + chunk.len() > MAX_ANSWER {
                return Err(HubError::Malformed { status });
            }
            answer.extend_from_slice(&chunk);
        }
        Ok((status, answer))
    }

    /// The failure in words. The request's URL is the public origin; no header is in it.
    fn unreachable(&self, error: &reqwest::Error) -> HubError {
        let reason = if error.is_timeout() {
            "it took longer than 30 seconds".to_owned()
        } else {
            deepest_cause(error)
        };
        HubError::Unreachable {
            origin: self.origin.as_text(),
            reason,
        }
    }
}

/// The request body as JSON, in a buffer that is wiped when the request drops.
fn encode(body: &impl Serialize) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(8 * 1024));
    serde_json::to_writer(&mut *bytes, body).expect("a GraphQL request body serialises");
    bytes
}

/// `Bearer <token>`, marked sensitive so the HTTP stack never shows it.
fn bearer(token: &Token) -> Result<HeaderValue, HubError> {
    let text = Zeroizing::new(format!("Bearer {}", token.expose()));
    let mut value = HeaderValue::from_str(&text).map_err(|_| HubError::Setup)?;
    value.set_sensitive(true);
    Ok(value)
}

/// The innermost cause of a request error, such as a refused connection or an unknown
/// certificate issuer.
fn deepest_cause(error: &reqwest::Error) -> String {
    let mut cause: &dyn std::error::Error = error;
    while let Some(inner) = cause.source() {
        cause = inner;
    }
    cause.to_string()
}
