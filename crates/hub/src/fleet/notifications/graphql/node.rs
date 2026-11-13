//! `NotificationChannel` and the types below it, as the API returns them. No secret is part of
//! any of them: a webhook shows its origin and header names, ntfy and e-mail only whether a
//! token or password is stored, Apprise its server, mode and tags (SEC-005, D-17).

use async_graphql::{Context, ID, Object, Result, SimpleObject, Union, dataloader::DataLoader};
use chrono::{DateTime, Utc};

use super::{
    delivery::{DeliveryNode, LastDeliveries},
    enums::{AppriseMode, ChannelKind, SmtpSecurity, WebhookMethod},
};
use crate::{
    api_errors::{internal, schema_dependency},
    fleet::notifications::{
        Channel, Error,
        channels::{AppriseSettings, EmailSettings, NtfySettings, Settings, WebhookSettings},
    },
};

#[derive(SimpleObject)]
#[graphql(name = "WebhookSettings")]
struct WebhookView {
    origin: String,
    method: WebhookMethod,
    header_names: Vec<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "NtfySettings")]
struct NtfyView {
    server: String,
    priority: i32,
    token_stored: bool,
}

#[derive(SimpleObject)]
#[graphql(name = "EmailSettings")]
struct EmailView {
    host: String,
    port: i32,
    security: SmtpSecurity,
    username: Option<String>,
    from: String,
    to: Vec<String>,
    password_stored: bool,
}

#[derive(SimpleObject)]
#[graphql(name = "AppriseSettings")]
struct AppriseView {
    server: String,
    mode: AppriseMode,
    tags: Vec<String>,
}

#[derive(Union)]
#[graphql(name = "ChannelSettings")]
enum SettingsView {
    Webhook(WebhookView),
    Ntfy(NtfyView),
    Email(EmailView),
    Apprise(AppriseView),
}

impl From<&Settings> for SettingsView {
    fn from(settings: &Settings) -> Self {
        match settings.clone() {
            Settings::Webhook(WebhookSettings {
                origin,
                method,
                header_names,
            }) => Self::Webhook(WebhookView {
                origin,
                method: method.into(),
                header_names,
            }),
            Settings::Ntfy(NtfySettings {
                server,
                priority,
                token_stored,
            }) => Self::Ntfy(NtfyView {
                server,
                priority: priority.into(),
                token_stored,
            }),
            Settings::Email(email) => Self::Email(email_view(email)),
            Settings::Apprise(AppriseSettings { server, mode, tags }) => {
                Self::Apprise(AppriseView {
                    server,
                    mode: mode.into(),
                    tags,
                })
            }
        }
    }
}

fn email_view(email: EmailSettings) -> EmailView {
    EmailView {
        host: email.host,
        port: email.port.into(),
        security: email.security.into(),
        username: email.username,
        from: email.from,
        to: email.to,
        password_stored: email.password_stored,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "NotificationTemplate")]
pub(in crate::fleet::notifications) struct TemplateView {
    pub(in crate::fleet::notifications) title: String,
    pub(in crate::fleet::notifications) body: String,
}

/// A stored channel with its settings read. No `Debug`: the channel holds sealed bytes.
pub(in crate::fleet::notifications) struct ChannelNode {
    pub(super) channel: Channel,
    pub(super) settings: Settings,
}

impl TryFrom<Channel> for ChannelNode {
    type Error = Error;

    /// # Errors
    /// `Unreadable` when the stored settings are not those of the channel's kind.
    fn try_from(channel: Channel) -> Result<Self, Error> {
        let settings = Settings::from_stored(channel.fields.kind, &channel.fields.settings)?;
        Ok(Self { channel, settings })
    }
}

#[Object(name = "NotificationChannel")]
impl ChannelNode {
    async fn id(&self) -> ID {
        ID(self.channel.id.to_string())
    }

    async fn name(&self) -> &str {
        &self.channel.fields.name
    }

    async fn kind(&self) -> ChannelKind {
        self.channel.fields.kind.into()
    }

    /// On for each new check (NOT-003).
    async fn on_by_default(&self) -> bool {
        self.channel.fields.on_by_default
    }

    async fn settings(&self) -> SettingsView {
        SettingsView::from(&self.settings)
    }

    /// Null: the installation's default template (NOT-007).
    async fn template(&self) -> Option<TemplateView> {
        let fields = &self.channel.fields;
        let title = fields.title_template.clone()?;
        let body = fields.body_template.clone()?;
        Some(TemplateView { title, body })
    }

    async fn last_delivery(&self, ctx: &Context<'_>) -> Result<Option<DeliveryNode>> {
        let loader = schema_dependency::<DataLoader<LastDeliveries>>(ctx)?;
        let delivery = loader.load_one(self.channel.id).await.map_err(internal)?;
        Ok(delivery.map(DeliveryNode::from))
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.channel.created_at
    }

    async fn updated_at(&self) -> DateTime<Utc> {
        self.channel.updated_at
    }
}
