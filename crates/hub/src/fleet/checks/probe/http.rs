//! HTTP probe: one request, every address and redirect hop judged by the address policy.
//!
//! The probe never decompresses a body, stores a cookie or uses an environment proxy. It
//! reads at most [`BODY_CAP`] bytes, and only when a keyword or JSON check needs them.
//! Header values, the body and credentials never reach a message (LOG-007).
//!
//! The caller validates the configuration (`probe` does). With `verify_tls = true` an
//! untrusted or expired certificate fails the request, so that result carries no
//! `cert_not_after`; with `verify_tls = false` the expiry is reported (CHK-007).

use std::{
    error::Error,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use reqwest::{
    Client, Method, RequestBuilder, Response, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    header::{HeaderName, HeaderValue},
    redirect::Policy,
    tls::TlsInfo,
};
use serde_json::Value;

use super::{
    ProbeEnv,
    outcome::{Failure, ProbeResult},
    resolve::allowed_addresses,
    tls,
};
use crate::fleet::{
    checks::config::{HttpAuth, HttpConfig, HttpMethod, HttpSecrets},
    targets::AddressPolicy,
};

/// The most body bytes read (1 MiB).
pub const BODY_CAP: usize = 1024 * 1024;

/// Resolves host names through the address policy; reqwest connects only to what it returns.
struct PolicyResolver(ProbeEnv);

impl Resolve for PolicyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let env = self.0.clone();
        Box::pin(async move {
            let ips = allowed_addresses(name.as_str(), None, &env)
                .await
                .map_err(|failure| Box::new(failure) as Box<dyn Error + Send + Sync>)?;
            Ok(Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0))) as Addrs)
        })
    }
}

/// Whether a URL may be requested: http or https, and an IP-literal host passes the policy.
/// Host names are judged when the resolver resolves them.
fn url_permitted(url: &Url, policy: &AddressPolicy) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .map_or(true, |ip| policy.permits(ip))
        })
}

/// Follows at most `max` redirects to permitted addresses. With secrets attached it refuses
/// a hop from HTTPS to HTTP: reqwest drops the secret headers on a change of host or port,
/// but not on a change of scheme alone (SEC-005).
fn redirect_policy(max: u8, policy: Arc<AddressPolicy>, secrets: bool) -> Policy {
    if max == 0 {
        return Policy::none();
    }
    Policy::custom(move |attempt| {
        let downgrade = attempt.url().scheme() == "http"
            && attempt.previous().iter().any(|url| url.scheme() == "https");
        if attempt.previous().len() > usize::from(max) {
            attempt.error(Failure::TooManyRedirects)
        } else if secrets && downgrade {
            attempt.error(Failure::InsecureRedirect)
        } else if !url_permitted(attempt.url(), &policy) {
            attempt.error(Failure::NotPermitted)
        } else {
            attempt.follow()
        }
    })
}

/// The failure a reqwest error carries: ours (from the resolver or redirect policy) first.
fn classify(error: &reqwest::Error) -> Failure {
    let mut source: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(cause) = source {
        if let Some(failure) = cause.downcast_ref::<Failure>() {
            return failure.clone();
        }
        if let Some(io) = cause.downcast_ref::<io::Error>() {
            return Failure::Connect(io.kind());
        }
        source = cause.source();
    }
    if error.is_timeout() {
        Failure::TimedOut
    } else {
        Failure::Request
    }
}

/// Whether the request carries a secret: authentication or a secret header.
fn has_secrets(config: &HttpConfig) -> bool {
    config.auth != HttpAuth::None || config.headers.iter().any(|header| header.value.is_none())
}

fn client(config: &HttpConfig, env: &ProbeEnv) -> Result<Client, Failure> {
    Client::builder()
        .use_rustls_tls()
        .user_agent("anima-hub")
        // Never take a proxy from the environment: it would send the request to a host the
        // address policy never judged. A child-process test sets the variable to prove it.
        .no_proxy()
        .redirect(redirect_policy(
            config.max_redirects,
            env.policy.clone(),
            has_secrets(config),
        ))
        .dns_resolver(Arc::new(PolicyResolver(env.clone())))
        .danger_accept_invalid_certs(!config.verify_tls)
        .timeout(Duration::from_millis(u64::from(config.timeout_ms)))
        .tls_info(true)
        .build()
        .map_err(|_| Failure::BadConfig)
}

fn method(method: HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Head => Method::HEAD,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
        HttpMethod::Options => Method::OPTIONS,
    }
}

fn secret_text(secret: &crate::fleet::credentials::Secret) -> Result<&str, Failure> {
    std::str::from_utf8(secret.expose()).map_err(|_| Failure::BadConfig)
}

fn request(
    client: &Client,
    config: &HttpConfig,
    secrets: Option<&HttpSecrets>,
    url: Url,
) -> Result<RequestBuilder, Failure> {
    let mut builder = client.request(method(config.method), url);
    let mut stored = secrets.map(|s| s.headers.iter());
    for header in &config.headers {
        let name =
            HeaderName::from_bytes(header.name.as_bytes()).map_err(|_| Failure::BadConfig)?;
        let (text, sensitive) = if let Some(text) = &header.value {
            (text.as_str(), false)
        } else {
            let secret = stored
                .as_mut()
                .and_then(Iterator::next)
                .ok_or(Failure::SecretMissing)?;
            (secret_text(secret)?, true)
        };
        let mut value = HeaderValue::from_str(text).map_err(|_| Failure::BadConfig)?;
        value.set_sensitive(sensitive);
        builder = builder.header(name, value);
    }
    builder = match &config.auth {
        HttpAuth::None => builder,
        HttpAuth::Basic { username } => {
            let password = secrets
                .and_then(|s| s.password.as_ref())
                .ok_or(Failure::SecretMissing)?;
            builder.basic_auth(username, Some(secret_text(password)?))
        }
        HttpAuth::Bearer => {
            let token = secrets
                .and_then(|s| s.bearer_token.as_ref())
                .ok_or(Failure::SecretMissing)?;
            builder.bearer_auth(secret_text(token)?)
        }
    };
    Ok(match &config.body {
        Some(body) => builder.body(body.clone()),
        None => builder,
    })
}

/// Reads at most [`BODY_CAP`] bytes, stopping as soon as the cap is reached.
async fn read_capped(response: &mut Response) -> Result<Vec<u8>, Failure> {
    let mut body = Vec::new();
    while body.len() < BODY_CAP {
        let Some(chunk) = response.chunk().await.map_err(|e| classify(&e))? else {
            break;
        };
        let room = BODY_CAP - body.len();
        body.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }
    Ok(body)
}

fn check_json(body: &[u8], query: &str, expected: Option<&str>) -> Result<(), Failure> {
    let json: Value = serde_json::from_slice(body).map_err(|_| Failure::BodyNotJson)?;
    let found = json.pointer(query).ok_or(Failure::JsonQueryMissing)?;
    let Some(expected) = expected else {
        return Ok(());
    };
    let text = match found {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    if text == expected {
        Ok(())
    } else {
        Err(Failure::JsonMismatch)
    }
}

/// Status, keyword and JSON checks on an answered request.
async fn judge(response: &mut Response, config: &HttpConfig) -> Result<(), Failure> {
    let code = response.status().as_u16();
    if !config
        .accepted_status
        .iter()
        .any(|range| (range.from..=range.to).contains(&code))
    {
        return Err(Failure::Status(code));
    }
    if config.keyword.is_none() && config.json_query.is_none() {
        return Ok(());
    }
    let body = read_capped(response).await?;
    if let Some(keyword) = &config.keyword
        && !String::from_utf8_lossy(&body).contains(keyword.as_str())
    {
        return Err(Failure::KeywordMissing);
    }
    match &config.json_query {
        Some(query) => check_json(&body, query, config.json_expected.as_deref()),
        None => Ok(()),
    }
}

fn certificate_expiry(response: &Response) -> Option<DateTime<Utc>> {
    let der = response.extensions().get::<TlsInfo>()?.peer_certificate()?;
    tls::validity(der).ok().map(|(_, not_after)| not_after)
}

pub(super) async fn probe(
    config: &HttpConfig,
    secrets: Option<&HttpSecrets>,
    env: &ProbeEnv,
) -> ProbeResult {
    let prepared = prepare(config, secrets, env);
    let request = match prepared {
        Ok(request) => request,
        Err(failure) => return ProbeResult::failed(&failure),
    };
    let started = Instant::now();
    let mut response = match request.send().await {
        Ok(response) => response,
        Err(error) => return ProbeResult::failed(&classify(&error)),
    };
    let mut result = match judge(&mut response, config).await {
        Ok(()) => ProbeResult::up(started.elapsed()),
        Err(failure) => ProbeResult::failed(&failure),
    };
    result.resolved_ip = response.remote_addr().map(|addr| addr.ip());
    result.cert_not_after = certificate_expiry(&response);
    result
}

fn prepare(
    config: &HttpConfig,
    secrets: Option<&HttpSecrets>,
    env: &ProbeEnv,
) -> Result<RequestBuilder, Failure> {
    let url = Url::parse(&config.url).map_err(|_| Failure::BadConfig)?;
    if !url_permitted(&url, &env.policy) {
        return Err(Failure::NotPermitted);
    }
    request(&client(config, env)?, config, secrets, url)
}
