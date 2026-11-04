//! HTTP check configuration: the stored part, free of secrets (LOG-007).

use serde::{Deserialize, Serialize};

use super::rules::{self, ConfigError, Reason, err, is_secret_header};

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
#[serde(deny_unknown_fields)]
pub struct StatusRange {
    pub from: u16,
    pub to: u16,
}

/// A request header. `value` is `None` for a secret header (Authorization,
/// Proxy-Authorization, Cookie): its value lives in [`HttpSecrets`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpHeader {
    pub name: String,
    pub value: Option<String>,
}

/// Which authentication the check uses; the secret part is in [`HttpSecrets`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "UPPERCASE", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HttpConfig {
    pub url: String,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: Vec<HttpHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default = "max_redirects")]
    pub max_redirects: u8,
    #[serde(default = "accepted")]
    pub accepted_status: Vec<StatusRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
