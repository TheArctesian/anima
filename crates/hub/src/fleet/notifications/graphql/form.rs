//! The channel input as a draft for the channel rules: defaults for null fields, the K1 enums
//! as the domain's, and every secret still a [`Secret`](crate::fleet::credentials::Secret).

use super::input::{
    AppriseShape, ChannelShape, DEFAULT_METHOD, DEFAULT_MODE, DEFAULT_NTFY, DEFAULT_ON_BY_DEFAULT,
    DEFAULT_PORT, DEFAULT_PRIORITY, DEFAULT_SECURITY, EmailShape, NtfyShape, SettingsShape,
    TemplateShape, WebhookShape,
};
use crate::fleet::notifications::channels::{
    AppriseDraft, Draft, EmailDraft, NtfyDraft, SettingsDraft, Template, WebhookDraft,
};

/// The draft of `input`. A null field with a K1 default takes that default, as an absent
/// one does.
pub(super) fn draft(input: ChannelShape) -> Draft {
    Draft {
        name: input.name,
        on_by_default: input.on_by_default.unwrap_or(DEFAULT_ON_BY_DEFAULT),
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
        method: input.method.unwrap_or(DEFAULT_METHOD).into(),
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
        priority: input.priority.unwrap_or(DEFAULT_PRIORITY),
    }
}

fn email(input: EmailShape) -> EmailDraft {
    EmailDraft {
        host: input.host,
        port: input.port.unwrap_or(DEFAULT_PORT),
        security: input.security.unwrap_or(DEFAULT_SECURITY).into(),
        username: input.username,
        password: input.password.map(|password| password.0),
        from: input.from,
        to: input.to,
    }
}

fn apprise(input: AppriseShape) -> AppriseDraft {
    AppriseDraft {
        server: input.server,
        mode: input.mode.unwrap_or(DEFAULT_MODE).into(),
        urls: input
            .urls
            .map(|urls| urls.into_iter().map(|url| url.0).collect()),
        key: input.key.map(|key| key.0),
        tags: input.tags.unwrap_or_default(),
    }
}
