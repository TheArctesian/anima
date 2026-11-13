//! One send through one channel: read its settings, open its secrets, render the template,
//! encode for the kind and hand the result to the egress (NOT-002, D-17, D-21). The dispatcher
//! and `sendTestNotification` both send through here, so a test notification takes the same
//! path as a real one.
//!
//! Everything before the egress is pure apart from the vault's decryption; the only I/O is the
//! connection, bounded by the egress time limit.

use super::{
    failure::Failure,
    outgoing::{Notice, Outgoing, outgoing},
};
use crate::fleet::{
    credentials::Vault,
    notifications::{
        Channel, DeliveryReason, InstallationSettings, NotificationKind,
        channels::Settings,
        egress::Egress,
        sealing,
        template::{self, Values},
    },
};

/// What a send needs besides the channel: the vault that opens its secrets and the one egress
/// path (P3-003).
#[derive(Clone)]
pub(in crate::fleet::notifications) struct Sender {
    vault: Vault,
    egress: Egress,
}

impl Sender {
    pub(in crate::fleet::notifications) const fn new(vault: Vault, egress: Egress) -> Self {
        Self { vault, egress }
    }

    /// Sends a `kind` notification with `values` through `channel`, worded by the channel's
    /// template or else by `defaults` (NOT-007).
    ///
    /// # Errors
    /// `SETUP_FAILED` when the stored settings or template cannot be read; `SECRET_UNREADABLE`
    /// when the sealed secrets do not open for these settings; nothing is sent then. Otherwise
    /// what [`outgoing`] or the egress refused.
    pub(in crate::fleet::notifications) async fn send(
        &self,
        channel: &Channel,
        (kind, values): (NotificationKind, &Values),
        defaults: &InstallationSettings,
    ) -> Result<(), Failure> {
        let fields = &channel.fields;
        let settings = Settings::from_stored(fields.kind, &fields.settings)
            .map_err(|_| Failure::of(DeliveryReason::SetupFailed))?;
        let secrets = fields
            .secrets
            .as_deref()
            .map(|sealed| sealing::open(&self.vault, channel.id, &settings, sealed))
            .transpose()
            .map_err(|_| Failure::of(DeliveryReason::SecretUnreadable))?;
        let (title, body) = match (&fields.title_template, &fields.body_template) {
            (Some(title), Some(body)) => (title, body),
            _ => (&defaults.title_template, &defaults.body_template),
        };
        let rendered = template::render(title, body, values)
            .map_err(|_| Failure::of(DeliveryReason::SetupFailed))?;
        let notice = Notice {
            kind,
            values,
            rendered: &rendered,
        };
        let request = outgoing(&settings, secrets, &notice)?;
        self.deliver(request).await
    }

    /// Hands `request` to the egress, which bounds it by its time limit.
    async fn deliver(&self, request: Outgoing) -> Result<(), Failure> {
        let sent = match request {
            Outgoing::Http(request) => self.egress.send_http(request).await,
            Outgoing::Mail {
                server,
                login,
                message,
            } => {
                self.egress
                    .send_mail(&server, login.as_ref(), &message)
                    .await
            }
        };
        sent.map_err(Failure::from)
    }
}
