//! What one notification puts on the wire for each kind of channel (NOT-002, D-17, D-18, N-4).
//! Pure: no connection and no clock; the secrets come in opened, and the result goes to the
//! egress unchanged.
//!
//! - **Webhook**: the channel's method to the whole sealed URL, `Content-Type:
//!   application/json`, the sealed header values under their stored names, and the JSON body
//!   of `encode::webhook`.
//! - **ntfy**: a POST to `<server>/<topic>` with the `Title` and `Priority` headers, the plain
//!   text body and, when a token is stored, `Authorization: Bearer <token>`.
//! - **Email**: a plain-text message from the channel's sender to its recipients, through its
//!   mail server, logging in only when a password is stored.
//! - **Apprise**: a POST of the `encode::apprise` object to `<server>/notify` with the stored
//!   URLs in `urls` (stateless), or to `<server>/notify/<key>` (a configuration stored in
//!   Apprise). Each stateless URL is checked by `permitted_apprise_url` again before every send,
//!   so a URL a database writer or an older version stored is refused (N-4). The object never
//!   holds `attach` or `attachment`. In key mode the hub cannot see where Apprise sends: the
//!   firewall around the Apprise service contains it (D-18, D-19). The Apprise server itself,
//!   like every other target here, passes the egress target rule (P3-003).
//!
//! An [`Outgoing`] holds secrets, so it has no `Debug`, and nothing here returns the target's
//! text (LOG-007).

use lettre::{
    Message,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use reqwest::{
    Method, Url,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::failure::Failure;
use crate::{
    credentials::Secret,
    notifications::{
        DeliveryReason, NotificationKind,
        channels::{
            AppriseMode, AppriseSecrets, AppriseSettings, EmailSettings, NtfySecrets, NtfySettings,
            Secrets, Settings, WebhookMethod, WebhookSecrets, WebhookSettings,
            permitted_apprise_url,
        },
        egress::{HttpRequest, SmtpServer},
        encode,
        template::{Rendered, Values},
    },
};

/// One notification as the template rendered it.
pub(in crate::notifications) struct Notice<'a> {
    pub(in crate::notifications) kind: NotificationKind,
    pub(in crate::notifications) values: &'a Values,
    pub(in crate::notifications) rendered: &'a Rendered,
}

/// A request ready for the egress. No `Debug`: it carries the channel's secrets.
pub(in crate::notifications) enum Outgoing {
    Http(HttpRequest),
    Mail {
        server: SmtpServer,
        login: Option<Credentials>,
        message: Box<Message>,
    },
}

/// What `notice` puts on the wire through a channel with `settings` and the opened `secrets`.
///
/// # Errors
/// `SECRET_UNREADABLE` when the secrets do not fit the settings; `BAD_ADDRESS` when a stored
/// URL or mail address does not parse; `TARGET_NOT_PERMITTED` when a stateless Apprise URL is
/// refused (N-4); `SETUP_FAILED` when a stored header cannot be sent.
pub(in crate::notifications) fn outgoing(
    settings: &Settings,
    secrets: Option<Secrets>,
    notice: &Notice<'_>,
) -> Result<Outgoing, Failure> {
    match (settings, secrets) {
        (Settings::Webhook(settings), Some(Secrets::Webhook(secrets))) => {
            webhook(settings, &secrets, notice)
        }
        (Settings::Ntfy(settings), Some(Secrets::Ntfy(secrets))) => {
            ntfy(settings, &secrets, notice)
        }
        (Settings::Email(settings), Some(Secrets::Email(secrets))) if settings.password_stored => {
            email(settings, Some(&secrets.password), notice)
        }
        (Settings::Email(settings), None) if !settings.password_stored => {
            email(settings, None, notice)
        }
        (Settings::Apprise(settings), Some(Secrets::Apprise(secrets))) => {
            apprise(settings, &secrets, notice)
        }
        _ => Err(Failure::of(DeliveryReason::SecretUnreadable)),
    }
}

/// The secret as text; every secret was sealed from a GraphQL string.
fn text(secret: &Secret) -> Result<&str, Failure> {
    std::str::from_utf8(secret.expose()).map_err(|_| Failure::of(DeliveryReason::SecretUnreadable))
}

const fn bad_address() -> Failure {
    Failure::of(DeliveryReason::BadAddress)
}

const fn setup_failed() -> Failure {
    Failure::of(DeliveryReason::SetupFailed)
}

/// `server` (an origin) with `segments` as its path, each percent-encoded as one segment.
fn under(server: &str, segments: &[&str]) -> Result<Url, Failure> {
    let mut url = Url::parse(server).map_err(|_| bad_address())?;
    url.path_segments_mut()
        .map_err(|()| bad_address())?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers
}

fn webhook(
    settings: &WebhookSettings,
    secrets: &WebhookSecrets,
    notice: &Notice<'_>,
) -> Result<Outgoing, Failure> {
    let url = Url::parse(text(&secrets.url)?).map_err(|_| bad_address())?;
    let mut headers = json_headers();
    for (name, value) in settings.header_names.iter().zip(&secrets.headers) {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| setup_failed())?;
        let value = HeaderValue::from_bytes(value.expose()).map_err(|_| setup_failed())?;
        headers.insert(name, value);
    }
    let body = encode::webhook(notice.rendered, notice.kind, notice.values);
    let method = match settings.method {
        WebhookMethod::Post => Method::POST,
        WebhookMethod::Put => Method::PUT,
    };
    Ok(Outgoing::Http(HttpRequest {
        method,
        url,
        headers,
        body: body.to_string().into_bytes(),
    }))
}

fn ntfy(
    settings: &NtfySettings,
    secrets: &NtfySecrets,
    notice: &Notice<'_>,
) -> Result<Outgoing, Failure> {
    let url = under(&settings.server, &[text(&secrets.topic)?])?;
    let message = encode::ntfy(notice.rendered, settings.priority).map_err(|_| setup_failed())?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert("title", message.title);
    headers.insert("priority", HeaderValue::from(u16::from(message.priority)));
    if let Some(token) = &secrets.token {
        let bearer = Zeroizing::new(format!("Bearer {}", text(token)?));
        let value = HeaderValue::from_str(&bearer).map_err(|_| setup_failed())?;
        headers.insert(AUTHORIZATION, value);
    }
    Ok(Outgoing::Http(HttpRequest {
        method: Method::POST,
        url,
        headers,
        body: message.body.into_bytes(),
    }))
}

fn mailbox(text: &str) -> Result<Mailbox, Failure> {
    text.parse().map_err(|_| bad_address())
}

fn email(
    settings: &EmailSettings,
    password: Option<&Secret>,
    notice: &Notice<'_>,
) -> Result<Outgoing, Failure> {
    let login = match (&settings.username, password) {
        (Some(username), Some(password)) => Some(Credentials::new(
            username.clone(),
            text(password)?.to_owned(),
        )),
        (_, None) => None,
        (None, Some(_)) => return Err(setup_failed()),
    };
    let wording = encode::email(notice.rendered);
    let mut builder = Message::builder()
        .from(mailbox(&settings.from)?)
        .subject(wording.subject)
        .header(ContentType::TEXT_PLAIN);
    for recipient in &settings.to {
        builder = builder.to(mailbox(recipient)?);
    }
    let message = builder.body(wording.body).map_err(|_| bad_address())?;
    let server = SmtpServer {
        host: settings.host.clone(),
        port: settings.port,
        security: settings.security,
    };
    Ok(Outgoing::Mail {
        server,
        login,
        message: Box::new(message),
    })
}

fn apprise(
    settings: &AppriseSettings,
    secrets: &AppriseSecrets,
    notice: &Notice<'_>,
) -> Result<Outgoing, Failure> {
    let mut body = encode::apprise(notice.rendered, notice.kind, &settings.tags);
    let url = match (settings.mode, secrets) {
        (AppriseMode::StatelessUrls, AppriseSecrets::Urls(urls)) => {
            let urls = urls.iter().map(text).collect::<Result<Vec<_>, _>>()?;
            if urls.is_empty() {
                return Err(Failure::of(DeliveryReason::SecretUnreadable));
            }
            if urls.iter().any(|url| permitted_apprise_url(url).is_err()) {
                return Err(Failure::of(DeliveryReason::TargetNotPermitted));
            }
            body.insert("urls".into(), json!(urls.join(",")));
            under(&settings.server, &["notify"])?
        }
        (AppriseMode::ConfigKey, AppriseSecrets::Key(key)) => {
            under(&settings.server, &["notify", text(key)?])?
        }
        _ => return Err(Failure::of(DeliveryReason::SecretUnreadable)),
    };
    Ok(Outgoing::Http(HttpRequest {
        method: Method::POST,
        url,
        headers: json_headers(),
        body: Value::Object(body).to_string().into_bytes(),
    }))
}
