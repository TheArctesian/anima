//! What a web interface mutation leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007, WEB-009).
//!
//! The audit target is the device (D-3). A `change` holds the interface id, the URL's origin
//! and a fingerprint, never the URL itself: a URL's path and query may carry a token. The
//! event names the interface by its label.

use std::sync::Arc;

use async_graphql::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde_json::{Value, json};
use sqlx::PgConnection;

use super::error::{internal, schema_dependency};
use crate::{
    clock::Clock,
    events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator, Severity,
        Source,
    },
    ids::{CheckId, DeviceId, OperatorId, WebInterfaceId},
    web_interfaces::WebInterface,
};

/// One mutation's entry in both logs.
pub(super) struct Entry {
    message: String,
    action: AuditAction,
    device: DeviceId,
    change: Value,
}

/// Who did it, from where and when; the same `at` stamps the store write and both entries.
pub(super) struct Trail<'a> {
    bus: &'a Bus,
    operator: Operator,
    client: ClientAddr,
    pub(super) now: DateTime<Utc>,
}

impl<'a> Trail<'a> {
    /// Reads the request's operator and address and the schema's bus and clock. The clock is
    /// read once.
    pub(super) fn from_context(ctx: &Context<'a>) -> Result<Self> {
        Ok(Self {
            bus: schema_dependency::<Bus>(ctx)?,
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            now: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        })
    }

    pub(super) fn operator(&self) -> OperatorId {
        self.operator.id
    }

    /// Writes the audit entry, then the event. The caller commits both with its own write.
    ///
    /// The event goes last because `events::record` publishes it on the bus at once: if the
    /// audit write fails, no subscriber has seen an event for a change that is rolled back.
    pub(super) async fn write(self, conn: &mut PgConnection, entry: Entry) -> Result<()> {
        let audit = NewAudit {
            operator: self.operator,
            action: entry.action,
            target: AuditTarget::Device(entry.device),
            client: self.client,
            change: entry.change,
        };
        events::audit(conn, self.now, audit)
            .await
            .map_err(internal)?;
        let event = NewEvent {
            severity: Severity::Info,
            source: Source::Device,
            device: None,
            message: entry.message,
        };
        events::record(conn, self.bus, self.now, event)
            .await
            .map(drop)
            .map_err(internal)
    }
}

/// `scheme://host:port` of a URL: no user name, path or query. Empty when it does not parse.
fn origin(url: &str) -> String {
    Url::parse(url).map_or_else(|_| String::new(), |u| u.origin().ascii_serialization())
}

fn fields(interface: &WebInterface) -> Value {
    json!({
        "interfaceId": interface.id.to_string(),
        "origin": origin(&interface.url),
        "label": interface.label,
        "openIn": interface.open_in.as_str(),
    })
}

pub(super) fn added(interface: &WebInterface) -> Entry {
    Entry {
        message: format!("Web interface {} added.", interface.label),
        action: AuditAction::Create,
        device: interface.device,
        change: fields(interface),
    }
}

/// `previousOrigin` and `previousLabel` appear when they changed. `pinDropped` is set when the new origin removed the certificate the operator accepted.
pub(super) fn updated(before: &WebInterface, after: &WebInterface) -> Entry {
    let mut change = fields(after);
    if before.pin.is_some() && after.pin.is_none() {
        change["pinDropped"] = json!(true);
    }
    if origin(&before.url) != origin(&after.url) {
        change["previousOrigin"] = json!(origin(&before.url));
    }
    if before.label != after.label {
        change["previousLabel"] = json!(before.label);
    }
    Entry {
        message: format!("Web interface {} updated.", after.label),
        action: AuditAction::Update,
        device: after.device,
        change,
    }
}

pub(super) fn deleted(id: WebInterfaceId, device: DeviceId) -> Entry {
    Entry {
        message: "Web interface deleted.".to_owned(),
        action: AuditAction::Delete,
        device,
        change: json!({ "interfaceId": id.to_string() }),
    }
}

/// `created` is false when the device already had an interface at that URL.
pub(super) fn suggestion_accepted(
    interface: &WebInterface,
    check: CheckId,
    created: bool,
) -> Entry {
    let mut change = fields(interface);
    change["checkId"] = json!(check.to_string());
    change["created"] = json!(created);
    Entry {
        message: format!("Web interface {} accepted from a check.", interface.label),
        action: AuditAction::Accept,
        device: interface.device,
        change,
    }
}

/// Hiding a suggestion changes what the device offers and removes nothing, so it is an
/// `update` of the device.
pub(super) fn suggestion_dismissed(check: CheckId, device: DeviceId) -> Entry {
    Entry {
        message: "Web interface suggestion dismissed.".to_owned(),
        action: AuditAction::Update,
        device,
        change: json!({ "checkId": check.to_string(), "dismissed": true }),
    }
}

/// The fingerprint is public (WEB-009).
pub(super) fn certificate_accepted(interface: &WebInterface) -> Entry {
    let mut change = fields(interface);
    if let Some(pin) = &interface.pin {
        change["fingerprint"] = json!(pin.fingerprint.as_str());
    }
    Entry {
        message: format!("Certificate of web interface {} accepted.", interface.label),
        action: AuditAction::Accept,
        device: interface.device,
        change,
    }
}
