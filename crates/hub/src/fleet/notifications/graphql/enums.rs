//! The K1 enums of the notifications API. The domain types stay free of GraphQL; these mirror
//! them, and the two directions are `From` conversions.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use crate::fleet::notifications::{
    ChannelKind as Kind, DeliveryReason as Reason, NotificationKind as Notification,
    channels::{AppriseMode as Mode, WebhookMethod as Method},
    egress::SmtpSecurity as Security,
};

macro_rules! mirror {
    ($domain:ty => $graphql:ident { $($variant:ident),+ $(,)? }) => {
        impl From<$domain> for $graphql {
            fn from(value: $domain) -> Self {
                match value { $(<$domain>::$variant => Self::$variant),+ }
            }
        }

        impl From<$graphql> for $domain {
            fn from(value: $graphql) -> Self {
                match value { $($graphql::$variant => Self::$variant),+ }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ChannelKind")]
pub(super) enum ChannelKind {
    Webhook,
    Ntfy,
    Email,
    Apprise,
}
mirror!(Kind => ChannelKind { Webhook, Ntfy, Email, Apprise });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "NotificationKind")]
pub(super) enum NotificationKind {
    Down,
    Up,
    StillDown,
    CertificateExpiry,
    DomainExpiry,
    Test,
}
mirror!(Notification => NotificationKind {
    Down, Up, StillDown, CertificateExpiry, DomainExpiry, Test
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "DeliveryReason")]
pub(super) enum DeliveryReason {
    TargetNotPermitted,
    BadAddress,
    SecretUnreadable,
    SetupFailed,
    ConnectionFailed,
    TlsFailed,
    TimedOut,
    HttpStatus,
    NoStarttls,
    SmtpRefused,
    NotUnderstood,
}
mirror!(Reason => DeliveryReason {
    TargetNotPermitted, BadAddress, SecretUnreadable, SetupFailed, ConnectionFailed, TlsFailed,
    TimedOut, HttpStatus, NoStarttls, SmtpRefused, NotUnderstood,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "WebhookMethod")]
pub(super) enum WebhookMethod {
    Post,
    Put,
}
mirror!(Method => WebhookMethod { Post, Put });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SmtpSecurity")]
pub(super) enum SmtpSecurity {
    /// TLS from the first byte.
    #[graphql(name = "TLS")]
    Implicit,
    #[graphql(name = "STARTTLS")]
    StartTls,
}
mirror!(Security => SmtpSecurity { Implicit, StartTls });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "AppriseMode")]
pub(super) enum AppriseMode {
    StatelessUrls,
    ConfigKey,
}
mirror!(Mode => AppriseMode { StatelessUrls, ConfigKey });
