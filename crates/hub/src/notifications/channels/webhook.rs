//! The webhook channel: a POST or PUT to an https URL, with optional headers (NOT-002, D-17,
//! D-18). The whole URL is a secret, because Discord and Slack keep the credential in its path,
//! and so is every header value; only the origin and the header names are shown.

use serde::{Deserialize, Serialize};

use super::{
    Kept, Secrets, Settings, Touched, WebhookMethod,
    outcome::{Outcome, unopened},
    rules::{Problems, https_url, origin, secret_text},
};
use crate::{api_errors::FieldError, credentials::Secret};

const URL: &str = "settings.webhook.url";
const HEADERS: &str = "settings.webhook.headers";

/// The most headers one webhook sends.
const HEADERS_MAX: usize = 20;
/// The longest header name, and the longest header value in bytes.
const NAME_MAX: usize = 128;
const VALUE_MAX: usize = 4096;

/// Header names the hub refuses (compared case-insensitively): the ones that frame the message
/// or the connection, which the HTTP client sets, and the ones a proxy in front of the target
/// reads to route the request elsewhere, rewrite its path or change its method. Only the URL
/// and the method field say where the request goes and what it does.
pub(super) const REFUSED_NAMES: [&str; 28] = [
    "host",
    "content-length",
    "content-encoding",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "proxy-connection",
    "proxy-authorization",
    "te",
    "trailer",
    "upgrade",
    "expect",
    "forwarded",
    "via",
    "max-forwards",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-forwarded-port",
    "x-forwarded-prefix",
    "x-forwarded-scheme",
    "x-forwarded-server",
    "x-real-ip",
    "x-host",
    "x-original-host",
    "x-original-url",
    "x-rewrite-url",
    "x-http-method-override",
];

/// What the API shows of a webhook.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::notifications) struct WebhookSettings {
    /// `https://host[:port]` of the sealed URL.
    pub(in crate::notifications) origin: String,
    pub(in crate::notifications) method: WebhookMethod,
    /// As the operator wrote them, in order; the values are sealed in the same order.
    pub(in crate::notifications) header_names: Vec<String>,
}

/// The whole URL and the header values, in the order of [`WebhookSettings::header_names`].
pub(in crate::notifications) struct WebhookSecrets {
    pub(in crate::notifications) url: Secret,
    pub(in crate::notifications) headers: Vec<Secret>,
}

/// A webhook as the operator sent it.
pub(in crate::notifications) struct WebhookDraft {
    /// `None` on update keeps the stored URL.
    pub(in crate::notifications) url: Option<Secret>,
    pub(in crate::notifications) method: WebhookMethod,
    /// `None` on update keeps the stored headers; an empty list removes them.
    pub(in crate::notifications) headers: Option<Vec<(String, Secret)>>,
}

/// The URL's origin, after the URL passed the rules.
fn checked_url(url: &Secret) -> Result<String, FieldError> {
    let text = secret_text(URL, url)?;
    if text.is_empty() {
        return Err(FieldError::new(URL, "Enter the webhook URL."));
    }
    https_url(URL, text).map(|url| origin(&url))
}

/// An RFC 9110 token that does not frame the request or reroute it at a proxy.
fn checked_name(name: &str) -> Result<(), &'static str> {
    let token = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b);
    if name.is_empty() {
        Err("Enter a name for each header.")
    } else if name.len() > NAME_MAX || !name.bytes().all(token) {
        Err("Use a header name of letters, digits and hyphens, at most 128 characters.")
    } else if REFUSED_NAMES
        .iter()
        .any(|refused| name.eq_ignore_ascii_case(refused))
    {
        Err("The hub sets this header itself, or a proxy would reroute the request by it.")
    } else {
        Ok(())
    }
}

/// A value with no control character but a tab (no line break, so no header injection).
fn checked_value(value: &Secret) -> Result<(), &'static str> {
    let text = std::str::from_utf8(value.expose()).map_err(|_| "Enter text.")?;
    if text.len() > VALUE_MAX {
        Err("Use at most 4096 bytes in a header value.")
    } else if text.chars().any(|c| c.is_control() && c != '\t') {
        Err("Remove line breaks and control characters from header values.")
    } else {
        Ok(())
    }
}

/// The names and values of a header list that passed the rules.
fn checked_headers(list: Vec<(String, Secret)>) -> Result<(Vec<String>, Vec<Secret>), FieldError> {
    let refuse = |message: &str| FieldError::new(HEADERS, message);
    if list.len() > HEADERS_MAX {
        return Err(refuse("Send at most 20 headers."));
    }
    let mut names: Vec<String> = Vec::with_capacity(list.len());
    let mut values = Vec::with_capacity(list.len());
    for (name, value) in list {
        checked_name(&name).map_err(refuse)?;
        checked_value(&value).map_err(refuse)?;
        if names.iter().any(|seen| seen.eq_ignore_ascii_case(&name)) {
            return Err(refuse("Send each header once."));
        }
        names.push(name);
        values.push(value);
    }
    Ok((names, values))
}

fn outcome(settings: WebhookSettings, secrets: WebhookSecrets, touched: Touched) -> Outcome {
    Outcome {
        settings: Settings::Webhook(settings),
        secrets: Some(Secrets::Webhook(secrets)),
        touched,
    }
}

/// The rules for a new webhook: a URL is required; headers are optional.
pub(super) fn new(draft: WebhookDraft) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let url = draft.url.unwrap_or_else(|| Secret::new(Vec::new()));
    let origin = problems.take(checked_url(&url));
    let headers = problems.take(checked_headers(draft.headers.unwrap_or_default()));
    let (Some(origin), Some((header_names, headers))) = (origin, headers) else {
        return Err(problems.into_errors());
    };
    let mut touched = Touched::default();
    touched.set("url");
    if !headers.is_empty() {
        touched.set("headers");
    }
    let settings = WebhookSettings {
        origin,
        method: draft.method,
        header_names,
    };
    Ok(outcome(settings, WebhookSecrets { url, headers }, touched))
}

/// The rules for a changed webhook (CR-14): a new origin needs the headers again; a null URL
/// or header list keeps the stored one.
pub(super) fn change(
    before: &WebhookSettings,
    stored: Kept<WebhookSecrets>,
    draft: WebhookDraft,
) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let new_origin = draft
        .url
        .as_ref()
        .and_then(|url| problems.take(checked_url(url)));
    let moved = new_origin
        .as_ref()
        .is_some_and(|origin| *origin != before.origin);
    let headers_kept = draft.headers.is_none() && !before.header_names.is_empty();
    if moved && headers_kept {
        problems.add(
            HEADERS,
            "The URL's origin changed: send every header again, or an empty list to remove them.",
        );
    }
    let new_headers = draft
        .headers
        .and_then(|list| problems.take(checked_headers(list)));
    problems.check()?;
    let (stored_url, stored_headers) = if draft.url.is_none() || headers_kept {
        let opened = stored.opened()?;
        (Some(opened.url), opened.headers)
    } else {
        (None, Vec::new())
    };
    let mut touched = Touched::default();
    if draft.url.is_some() {
        touched.set("url");
    }
    let url = draft.url.or(stored_url).ok_or_else(unopened)?;
    let (header_names, headers) = match new_headers {
        Some((names, values)) => {
            if !values.is_empty() {
                touched.set("headers");
            } else if !before.header_names.is_empty() {
                touched.cleared("headers");
            }
            (names, values)
        }
        None if headers_kept => (before.header_names.clone(), stored_headers),
        None => (Vec::new(), Vec::new()),
    };
    let settings = WebhookSettings {
        origin: new_origin.unwrap_or_else(|| before.origin.clone()),
        method: draft.method,
        header_names,
    };
    Ok(outcome(settings, WebhookSecrets { url, headers }, touched))
}
