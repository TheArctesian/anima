//! The secrets of a channel, typed per kind, and their sealed JSON layout (SEC-003).
//!
//! Like [`Secret`] they have no `Debug`, `Display`, `Clone` or `Serialize`; [`Wire`] is the
//! transient, borrowed form that sealing writes and reads.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::{AppriseMode, AppriseSecrets, EmailSecrets, NtfySecrets, Settings, WebhookSecrets};
use crate::fleet::credentials::Secret;

/// The secrets of one channel.
pub(in crate::fleet::notifications) enum Secrets {
    Webhook(WebhookSecrets),
    Ntfy(NtfySecrets),
    Email(EmailSecrets),
    Apprise(AppriseSecrets),
}

/// The sealed layout of [`Secrets`]: one optional field per secret, so each kind fills its
/// own. Copies are borrowed from the secrets, or from the opened plaintext, and short-lived.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet::notifications) struct Wire<'a> {
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    url: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Vec::is_empty")]
    headers: Vec<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    topic: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    token: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    password: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Vec::is_empty")]
    urls: Vec<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    key: Option<Cow<'a, str>>,
}

/// The secret as text for sealing; bytes that are not UTF-8 are replaced, never dropped. Every
/// secret came from a GraphQL string, so none is replaced in practice.
fn lossy(secret: &Secret) -> Cow<'_, str> {
    String::from_utf8_lossy(secret.expose())
}

fn own(text: Cow<'_, str>) -> Secret {
    Secret::from(text.into_owned())
}

impl Wire<'_> {
    /// The most bytes this can take as JSON: every byte escaped as `\uXXXX`, plus quotes,
    /// commas and the field names.
    pub(in crate::fleet::notifications) fn worst_len(&self) -> usize {
        let parts = [
            &self.url,
            &self.topic,
            &self.token,
            &self.password,
            &self.key,
        ]
        .into_iter()
        .flatten()
        .chain(&self.headers)
        .chain(&self.urls);
        parts.map(|part| part.len() * 6 + 8).sum::<usize>() + 128
    }

    /// Which fields hold a value, in declaration order.
    fn present(&self) -> [bool; 7] {
        [
            self.url.is_some(),
            !self.headers.is_empty(),
            self.topic.is_some(),
            self.token.is_some(),
            self.password.is_some(),
            !self.urls.is_empty(),
            self.key.is_some(),
        ]
    }
}

impl Secrets {
    /// The borrowed sealed layout.
    pub(in crate::fleet::notifications) fn to_wire(&self) -> Wire<'_> {
        match self {
            Self::Webhook(secrets) => Wire {
                url: Some(lossy(&secrets.url)),
                headers: secrets.headers.iter().map(lossy).collect(),
                ..Wire::default()
            },
            Self::Ntfy(secrets) => Wire {
                topic: Some(lossy(&secrets.topic)),
                token: secrets.token.as_ref().map(lossy),
                ..Wire::default()
            },
            Self::Email(secrets) => Wire {
                password: Some(lossy(&secrets.password)),
                ..Wire::default()
            },
            Self::Apprise(AppriseSecrets::Urls(urls)) => Wire {
                urls: urls.iter().map(lossy).collect(),
                ..Wire::default()
            },
            Self::Apprise(AppriseSecrets::Key(key)) => Wire {
                key: Some(lossy(key)),
                ..Wire::default()
            },
        }
    }

    /// The secrets in `wire`, if they have exactly the shape `settings` needs: the kind's
    /// required secrets, one value per webhook header, a token or password exactly when the
    /// settings say one is stored, the Apprise secret of the settings' mode, and nothing else.
    pub(in crate::fleet::notifications) fn from_wire(
        settings: &Settings,
        wire: Wire<'_>,
    ) -> Option<Self> {
        let present = wire.present();
        let (secrets, expected) = match settings {
            Settings::Webhook(webhook) => {
                if wire.headers.len() != webhook.header_names.len() {
                    return None;
                }
                let headers = !webhook.header_names.is_empty();
                let secrets = Self::Webhook(WebhookSecrets {
                    url: own(wire.url?),
                    headers: wire.headers.into_iter().map(own).collect(),
                });
                (secrets, [true, headers, false, false, false, false, false])
            }
            Settings::Ntfy(ntfy) => {
                let secrets = Self::Ntfy(NtfySecrets {
                    topic: own(wire.topic?),
                    token: wire.token.map(own),
                });
                let token = ntfy.token_stored;
                (secrets, [false, false, true, token, false, false, false])
            }
            Settings::Email(email) if email.password_stored => {
                let password = own(wire.password?);
                let secrets = Self::Email(EmailSecrets { password });
                (secrets, [false, false, false, false, true, false, false])
            }
            Settings::Email(_) => return None,
            Settings::Apprise(apprise) => match apprise.mode {
                AppriseMode::StatelessUrls => {
                    let urls = wire.urls.into_iter().map(own).collect();
                    let secrets = Self::Apprise(AppriseSecrets::Urls(urls));
                    (secrets, [false, false, false, false, false, true, false])
                }
                AppriseMode::ConfigKey => {
                    let secrets = Self::Apprise(AppriseSecrets::Key(own(wire.key?)));
                    (secrets, [false, false, false, false, false, false, true])
                }
            },
        };
        (present == expected).then_some(secrets)
    }

    pub(super) fn into_webhook(self) -> Option<WebhookSecrets> {
        match self {
            Self::Webhook(secrets) => Some(secrets),
            _ => None,
        }
    }

    pub(super) fn into_ntfy(self) -> Option<NtfySecrets> {
        match self {
            Self::Ntfy(secrets) => Some(secrets),
            _ => None,
        }
    }

    pub(super) fn into_email(self) -> Option<EmailSecrets> {
        match self {
            Self::Email(secrets) => Some(secrets),
            _ => None,
        }
    }

    pub(super) fn into_apprise(self) -> Option<AppriseSecrets> {
        match self {
            Self::Apprise(secrets) => Some(secrets),
            _ => None,
        }
    }
}
