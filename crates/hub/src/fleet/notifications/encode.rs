//! What each kind of channel sends for a rendered notification (NOT-007, D-21, criterion 10).
//! Pure: nothing here connects, reads a secret or looks at a clock; the dispatcher sends what
//! these functions return.
//!
//! - **Webhook**: a JSON document built with `serde_json`, never by joining strings. Its keys
//!   are `kind`, `title`, `body`, `status`, `previous`, `device`, `check`, `message`, `at` and
//!   `days`. Every key is always present; `message`, `previous` and the like are `null` when
//!   the notification has no such value, and `days` is a whole number or `null`.
//! - **ntfy**: the title as a header value on one line, the body as plain text, and the
//!   channel's priority.
//! - **Email**: a subject on one line and a plain-text body, for the mail message.
//! - **Apprise**: the JSON object for `/notify` with `title`, `body`, `type`, `format` and the
//!   channel's `tag`. It carries no `urls`, no `attach` and no `attachment`: the dispatcher
//!   adds the stored URLs in stateless mode, and nothing in a notification can add a link.
//!
//! A value or template that holds line breaks, NUL, bidirectional overrides or braces reaches
//! no channel in a form that changes its structure (`text`).

#[cfg(test)]
mod tests;
mod text;

use reqwest::header::{HeaderValue, InvalidHeaderValue};
use serde_json::{Map, Value, json};

use self::text::{plain_body, single_line};
use super::{
    NotificationKind,
    template::{Rendered, Values},
};

/// A title that cannot be sent as an HTTP header.
#[derive(Debug, thiserror::Error)]
#[error("the title cannot be sent as a header")]
pub(super) struct NotAHeader(#[from] InvalidHeaderValue);

/// The JSON body of a webhook (the shape is described above).
pub(super) fn webhook(rendered: &Rendered, kind: NotificationKind, values: &Values) -> Value {
    json!({
        "kind": kind.as_str(),
        "title": single_line(&rendered.title),
        "body": plain_body(&rendered.body),
        "status": values.status,
        "previous": values.previous,
        "device": values.device,
        "check": values.check,
        "message": values.message,
        "at": values.at,
        "days": values.days,
    })
}

/// A ntfy message: `Title` header, plain-text body, `Priority` 1 to 5.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Ntfy {
    pub(super) title: HeaderValue,
    pub(super) body: String,
    pub(super) priority: u8,
}

/// The ntfy message. The title is UTF-8; ntfy reads it as it arrives.
///
/// # Errors
/// `NotAHeader` when the one-line title is still not a header value. It cannot happen after
/// `single_line`; the type says so instead of hiding the case.
pub(super) fn ntfy(rendered: &Rendered, priority: u8) -> Result<Ntfy, NotAHeader> {
    Ok(Ntfy {
        title: HeaderValue::from_bytes(single_line(&rendered.title).as_bytes())?,
        body: plain_body(&rendered.body),
        priority: priority.clamp(1, 5),
    })
}

/// The subject and plain-text body of an e-mail.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Email {
    pub(super) subject: String,
    pub(super) body: String,
}

pub(super) fn email(rendered: &Rendered) -> Email {
    Email {
        subject: single_line(&rendered.title),
        body: plain_body(&rendered.body),
    }
}

/// The Apprise `type` of a notification: how the target colours it.
const fn apprise_type(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Down | NotificationKind::StillDown => "failure",
        NotificationKind::Up => "success",
        NotificationKind::CertificateExpiry | NotificationKind::DomainExpiry => "warning",
        NotificationKind::Test => "info",
    }
}

/// The JSON object for Apprise's `/notify`. `tag` is the channel's tags joined by commas
/// (Apprise's OR), left out when the channel has none.
pub(super) fn apprise(
    rendered: &Rendered,
    kind: NotificationKind,
    tags: &[String],
) -> Map<String, Value> {
    let mut body = Map::new();
    body.insert("title".into(), json!(single_line(&rendered.title)));
    body.insert("body".into(), json!(plain_body(&rendered.body)));
    body.insert("type".into(), json!(apprise_type(kind)));
    body.insert("format".into(), json!("text"));
    if !tags.is_empty() {
        let tags: Vec<_> = tags.iter().map(|tag| single_line(tag)).collect();
        body.insert("tag".into(), json!(tags.join(",")));
    }
    body
}
