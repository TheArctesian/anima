//! The channel input as a draft for the channel rules: defaults for null fields, the K1 enums
//! as the domain's, and every secret still a [`Secret`](crate::credentials::Secret).

use super::input::{
    AppriseShape, ChannelShape, DEFAULT_NTFY, EmailShape, NtfyShape, SettingsShape, TemplateShape,
    WebhookShape,
};
use crate::notifications::{
    channels::{
        AppriseDraft, AppriseMode, Draft, EmailDraft, NtfyDraft, SettingsDraft, Template,
        WebhookDraft, WebhookMethod,
    },
    egress::SmtpSecurity,
};

/// The draft of `input`. A null field with a K1 default takes that default, as an absent
/// one does.
pub(super) fn draft(input: ChannelShape) -> Draft {
    Draft {
        name: input.name,
        on_by_default: input.on_by_default.unwrap_or(false),
        settings: settings(input.settings),
        template: input
            .template
            .map(|TemplateShape { title, body }| Template { title, body }),
    }
}

fn settings(input: SettingsShape) -> SettingsDraft {
    match input {
        SettingsShape::Webhook(input) => SettingsDraft::Webhook(webhook(input)),
        SettingsShape::Ntfy(input) => SettingsDraft::Ntfy(ntfy(input)),
        SettingsShape::Email(input) => SettingsDraft::Email(email(input)),
        SettingsShape::Apprise(input) => SettingsDraft::Apprise(apprise(input)),
    }
}

fn webhook(input: WebhookShape) -> WebhookDraft {
    WebhookDraft {
        url: input.url.map(|url| url.0),
        method: input.method.map_or(WebhookMethod::Post, Into::into),
        headers: input.headers.map(|headers| {
            headers
                .into_iter()
                .map(|header| (header.0.name, header.0.value.0))
                .collect()
        }),
    }
}

fn ntfy(input: NtfyShape) -> NtfyDraft {
    NtfyDraft {
        server: input.server.unwrap_or_else(|| DEFAULT_NTFY.to_owned()),
        topic: input.topic.map(|topic| topic.0),
        token: input.token.map(|token| token.0),
        priority: input.priority.unwrap_or(3),
    }
}

fn email(input: EmailShape) -> EmailDraft {
    EmailDraft {
        host: input.host,
        port: input.port.unwrap_or(587),
        security: input.security.map_or(SmtpSecurity::StartTls, Into::into),
        username: input.username,
        password: input.password.map(|password| password.0),
        from: input.from,
        to: input.to,
    }
}

fn apprise(input: AppriseShape) -> AppriseDraft {
    AppriseDraft {
        server: input.server,
        mode: input.mode.map_or(AppriseMode::StatelessUrls, Into::into),
        urls: input
            .urls
            .map(|urls| urls.into_iter().map(|url| url.0).collect()),
        key: input.key.map(|key| key.0),
        tags: input.tags.unwrap_or_default(),
    }
}
