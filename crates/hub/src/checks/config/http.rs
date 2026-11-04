//! HTTP check configuration: the public part that is stored as JSON, and the secrets that
//! are kept apart (LOG-007).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::rules::{self, ConfigError, Reason, err, is_secret_header};
use crate::credentials::Secret;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}

/// An inclusive range of accepted status codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusRange {
    pub from: u16,
    pub to: u16,
}

/// A request header. `value` is `None` for a secret header (Authorization,
/// Proxy-Authorization, Cookie): its value lives in [`HttpSecrets`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: Option<String>,
}

/// Which authentication the check uses; the secret part is in [`HttpSecrets`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HttpAuth {
    #[default]
    None,
    Basic {
        username: String,
    },
    Bearer,
}

const fn max_redirects() -> u8 {
    10
}
fn accepted() -> Vec<StatusRange> {
    vec![StatusRange { from: 200, to: 299 }]
}
const fn verify_tls() -> bool {
    true
}
const fn timeout_ms() -> u32 {
    10_000
}

/// The stored, secret-free part of an HTTP check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpConfig {
    pub url: String,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: Vec<HttpHeader>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default = "max_redirects")]
    pub max_redirects: u8,
    #[serde(default = "accepted")]
    pub accepted_status: Vec<StatusRange>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub json_query: Option<String>,
    #[serde(default)]
    pub json_expected: Option<String>,
    #[serde(default)]
    pub auth: HttpAuth,
    #[serde(default = "verify_tls")]
    pub verify_tls: bool,
    #[serde(default = "timeout_ms")]
    pub timeout_ms: u32,
}

impl HttpConfig {
    /// A GET check of `url` with the contract defaults (accepts 200 to 299).
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Get,
            headers: Vec::new(),
            body: None,
            max_redirects: max_redirects(),
            accepted_status: accepted(),
            keyword: None,
            json_query: None,
            json_expected: None,
            auth: HttpAuth::None,
            verify_tls: true,
            timeout_ms: timeout_ms(),
        }
    }

    /// # Errors
    /// The first refused field; the error never repeats the value.
    pub fn validate(&self) -> Result<(), ConfigError> {
        rules::timeout("timeoutMs", self.timeout_ms)?;
        self.validate_url()?;
        self.validate_headers()?;
        if self.max_redirects > rules::REDIRECTS_MAX {
            let max = u32::from(rules::REDIRECTS_MAX);
            return Err(err("maxRedirects", Reason::OutOfRange { min: 0, max }));
        }
        self.validate_status()?;
        self.validate_body_checks()?;
        match &self.auth {
            HttpAuth::Basic { username }
                if username.is_empty() || username.contains(':') || username.len() > 256 =>
            {
                Err(err("auth", Reason::Invalid))
            }
            _ => Ok(()),
        }
    }

    fn validate_url(&self) -> Result<(), ConfigError> {
        rules::max_len("url", &self.url, rules::URL_MAX)?;
        let url = reqwest::Url::parse(&self.url).map_err(|_| err("url", Reason::Invalid))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(err("url", Reason::Scheme));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(err("url", Reason::Userinfo));
        }
        if url.host().is_none() {
            Err(err("url", Reason::Invalid))
        } else {
            Ok(())
        }
    }

    fn validate_headers(&self) -> Result<(), ConfigError> {
        if self.headers.len() > rules::HEADERS_MAX {
            return Err(err(
                "headers",
                Reason::TooMany {
                    max: rules::HEADERS_MAX,
                },
            ));
        }
        for header in &self.headers {
            rules::header_name("headers", &header.name)?;
            match &header.value {
                // A secret value must never sit in the stored JSON.
                Some(_) if is_secret_header(&header.name) => {
                    return Err(err("headers", Reason::Invalid));
                }
                Some(value) => rules::header_value("headers", value)?,
                None if !is_secret_header(&header.name) => {
                    return Err(err("headers", Reason::Missing));
                }
                None => {}
            }
        }
        if let Some(body) = &self.body {
            rules::max_len("body", body, rules::BODY_MAX)?;
        }
        Ok(())
    }

    fn validate_status(&self) -> Result<(), ConfigError> {
        if self.accepted_status.is_empty() {
            return Err(err("acceptedStatus", Reason::Empty));
        }
        if self.accepted_status.len() > 20 {
            return Err(err("acceptedStatus", Reason::TooMany { max: 20 }));
        }
        let ok = |r: &StatusRange| (100..=599).contains(&r.from) && r.from <= r.to && r.to <= 599;
        if self.accepted_status.iter().all(ok) {
            Ok(())
        } else {
            Err(err(
                "acceptedStatus",
                Reason::OutOfRange { min: 100, max: 599 },
            ))
        }
    }

    fn validate_body_checks(&self) -> Result<(), ConfigError> {
        if let Some(keyword) = &self.keyword {
            rules::max_len("keyword", keyword, rules::KEYWORD_MAX)?;
        }
        if let Some(query) = &self.json_query {
            rules::json_pointer("jsonQuery", query)?;
        }
        if let Some(expected) = &self.json_expected {
            rules::max_len("jsonExpected", expected, rules::KEYWORD_MAX)?;
            if self.json_query.is_none() {
                return Err(err("jsonExpected", Reason::Missing));
            }
        }
        Ok(())
    }
}

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

fn text(secret: &Secret) -> &str {
    // Secrets are built from `String`s, so the bytes are valid UTF-8.
    std::str::from_utf8(secret.expose()).unwrap_or_default()
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
            password: self.password.as_ref().map(|s| Cow::Borrowed(text(s))),
            bearer_token: self.bearer_token.as_ref().map(|s| Cow::Borrowed(text(s))),
            headers: self
                .headers
                .iter()
                .map(|s| Cow::Borrowed(text(s)))
                .collect(),
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
                rules::header_value("headers", text(&value))?;
                secrets.headers.push(value);
                headers.push(HttpHeader { name, value: None });
            } else {
                headers.push(HttpHeader {
                    name,
                    value: Some(text(&value).to_owned()),
                });
            }
        }
        let auth = match self.credentials {
            HttpCredentials::None => HttpAuth::None,
            HttpCredentials::Basic { username, password } => {
                secrets.password = Some(password);
                HttpAuth::Basic { username }
            }
            HttpCredentials::Bearer(token) => {
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
