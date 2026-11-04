//! The secret-bearing form of an HTTP check: what the operator supplies, and the secrets
//! that are kept apart from the stored JSON (LOG-007).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::{
    http::{HttpAuth, HttpConfig, HttpHeader, HttpMethod, StatusRange},
    rules::{self, ConfigError, Reason, err, is_secret_header},
};
use crate::credentials::Secret;

/// How the request authenticates, with the secret in hand.
pub enum HttpCredentials {
    None,
    Basic { username: String, password: Secret },
    Bearer(Secret),
}

/// An HTTP check as an operator supplies it: every header value and the credentials are
/// still in hand. It prints nothing (it has no `Debug`); [`HttpInput::split`] separates the
/// stored part from the secrets.
pub struct HttpInput {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Vec<(String, Secret)>,
    pub body: Option<String>,
    pub max_redirects: u8,
    pub accepted_status: Vec<StatusRange>,
    pub keyword: Option<String>,
    pub json_query: Option<String>,
    pub json_expected: Option<String>,
    pub credentials: HttpCredentials,
    pub verify_tls: bool,
    pub timeout_ms: u32,
}

/// The secrets of an HTTP check, kept apart from the stored JSON.
///
/// Like [`Secret`] it has no `Debug`, `Display`, `Clone` or `Serialize`. `headers` holds the
/// values of the secret headers in the order of the `None` values of [`HttpConfig::headers`].
#[derive(Default)]
pub struct HttpSecrets {
    pub password: Option<Secret>,
    pub bearer_token: Option<Secret>,
    pub headers: Vec<Secret>,
}

/// The sealed layout of [`HttpSecrets`]; copies are transient and short-lived.
#[derive(Serialize, Deserialize)]
struct Wire<'a> {
    #[serde(borrow)]
    password: Option<Cow<'a, str>>,
    #[serde(borrow)]
    bearer_token: Option<Cow<'a, str>>,
    #[serde(borrow)]
    headers: Vec<Cow<'a, str>>,
}

/// The secret as text, for a field that must be UTF-8. The error names the field only.
fn text<'a>(field: &'static str, secret: &'a Secret) -> Result<&'a str, ConfigError> {
    std::str::from_utf8(secret.expose()).map_err(|_| err(field, Reason::Invalid))
}

/// The secret as text for sealing; bytes that are not UTF-8 are replaced, never dropped.
fn lossy(secret: &Secret) -> Cow<'_, str> {
    String::from_utf8_lossy(secret.expose())
}

impl HttpSecrets {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.password.is_none() && self.bearer_token.is_none() && self.headers.is_empty()
    }

    /// The bytes to seal into the `secrets_sealed` column.
    #[must_use]
    pub fn encode(&self) -> Secret {
        let wire = Wire {
            password: self.password.as_ref().map(lossy),
            bearer_token: self.bearer_token.as_ref().map(lossy),
            headers: self.headers.iter().map(lossy).collect(),
        };
        Secret::new(serde_json::to_vec(&wire).unwrap_or_default())
    }

    /// Reads back what [`HttpSecrets::encode`] wrote.
    ///
    /// # Errors
    /// The bytes are not in the sealed layout; the error holds no content.
    pub fn decode(bytes: &Secret) -> Result<Self, ConfigError> {
        let wire: Wire<'_> =
            serde_json::from_slice(bytes.expose()).map_err(|_| err("secrets", Reason::Invalid))?;
        let own = |c: Cow<'_, str>| Secret::from(c.into_owned());
        Ok(Self {
            password: wire.password.map(own),
            bearer_token: wire.bearer_token.map(own),
            headers: wire.headers.into_iter().map(own).collect(),
        })
    }
}

impl HttpInput {
    /// Separates the stored configuration from the secrets and validates both.
    ///
    /// # Errors
    /// The first refused field; the error never repeats the value.
    pub fn split(self) -> Result<(HttpConfig, HttpSecrets), ConfigError> {
        let mut secrets = HttpSecrets::default();
        let mut headers = Vec::with_capacity(self.headers.len());
        for (name, value) in self.headers {
            if is_secret_header(&name) {
                rules::header_name("headers", &name)?;
                rules::header_value("headers", text("headers", &value)?)?;
                secrets.headers.push(value);
                headers.push(HttpHeader { name, value: None });
            } else {
                headers.push(HttpHeader {
                    name,
                    value: Some(text("headers", &value)?.to_owned()),
                });
            }
        }
        let auth = match self.credentials {
            HttpCredentials::None => HttpAuth::None,
            HttpCredentials::Basic { username, password } => {
                text("auth", &password)?;
                secrets.password = Some(password);
                HttpAuth::Basic { username }
            }
            HttpCredentials::Bearer(token) => {
                text("auth", &token)?;
                secrets.bearer_token = Some(token);
                HttpAuth::Bearer
            }
        };
        let config = HttpConfig {
            url: self.url,
            method: self.method,
            headers,
            body: self.body,
            max_redirects: self.max_redirects,
            accepted_status: self.accepted_status,
            keyword: self.keyword,
            json_query: self.json_query,
            json_expected: self.json_expected,
            auth,
            verify_tls: self.verify_tls,
            timeout_ms: self.timeout_ms,
        };
        config.validate()?;
        Ok((config, secrets))
    }

    /// The reverse of [`HttpInput::split`], to edit a stored check.
    ///
    /// # Errors
    /// A secret that the configuration needs is missing.
    pub fn join(config: HttpConfig, secrets: HttpSecrets) -> Result<Self, ConfigError> {
        let mut stored = secrets.headers.into_iter();
        let mut headers = Vec::with_capacity(config.headers.len());
        for header in config.headers {
            let value = match header.value {
                Some(value) => Secret::from(value),
                None => stored.next().ok_or(err("headers", Reason::Missing))?,
            };
            headers.push((header.name, value));
        }
        let credentials = match config.auth {
            HttpAuth::None => HttpCredentials::None,
            HttpAuth::Basic { username } => HttpCredentials::Basic {
                username,
                password: secrets.password.ok_or(err("auth", Reason::Missing))?,
            },
            HttpAuth::Bearer => {
                HttpCredentials::Bearer(secrets.bearer_token.ok_or(err("auth", Reason::Missing))?)
            }
        };
        Ok(Self {
            url: config.url,
            method: config.method,
            headers,
            body: config.body,
            max_redirects: config.max_redirects,
            accepted_status: config.accepted_status,
            keyword: config.keyword,
            json_query: config.json_query,
            json_expected: config.json_expected,
            credentials,
            verify_tls: config.verify_tls,
            timeout_ms: config.timeout_ms,
        })
    }
}
