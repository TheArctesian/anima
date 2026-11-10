//! Why one send failed, as the delivery log keeps it, and whether trying again can help
//! (CR-14 item 5, D-16). Pure.
//!
//! A failure is a reason word and, for an HTTP status or an SMTP reply, its code; never the
//! target's text (LOG-007).
//!
//! | Reason | Code | Permanent |
//! | --- | --- | --- |
//! | `TARGET_NOT_PERMITTED` (refused host, refused Apprise URL) | — | yes |
//! | `TARGET_NOT_PERMITTED` (unresolved host) | — | no |
//! | `BAD_ADDRESS` (not an https URL; an unreadable mail address) | — | yes |
//! | `SECRET_UNREADABLE` (sealed secrets that do not open or do not fit) | — | yes |
//! | `SETUP_FAILED` (unreadable settings or template; the hub's own TLS or client setup) | — | yes |
//! | `HTTP_STATUS` 4xx but 408 and 429 | status | yes |
//! | `HTTP_STATUS` 408, 429, 1xx, 3xx, 5xx | status | no |
//! | `SMTP_REFUSED` 5xx (RFC 5321: a permanent negative reply) | reply | yes |
//! | `SMTP_REFUSED` 4xx (a transient negative reply) | reply | no |
//! | `CONNECTION_FAILED`, `TLS_FAILED`, `TIMED_OUT`, `NO_STARTTLS`, `NOT_UNDERSTOOD` | — | no |
//!
//! A permanent failure is one the same send would meet again until the operator changes the
//! channel or the target's owner changes the target, so it is given up at once. A host that
//! resolves to an address outside the egress allow list is permanent. A host that does not
//! resolve is not (R-103): a lookup failure is usually brief, so it is retried on the normal
//! schedule. It keeps the reason word `TARGET_NOT_PERMITTED` and names no address, so the
//! delivery log says no more than its retries do.

use chrono::{DateTime, Utc};

use crate::{
    ids::DeliveryId,
    notifications::{Channel, Delivery, DeliveryReason, NotificationKind, egress::SendError},
    targets::TargetError,
};

/// One failed send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::notifications) struct Failure {
    pub(in crate::notifications) reason: DeliveryReason,
    /// The HTTP status for `HttpStatus` and the SMTP reply code for `SmtpRefused`; `None`
    /// for every other reason.
    pub(in crate::notifications) code: Option<i32>,
    /// The host did not resolve: `TARGET_NOT_PERMITTED`, but retried (R-103).
    unresolved: bool,
}

impl Failure {
    /// A failure whose reason has no code.
    pub(in crate::notifications) const fn of(reason: DeliveryReason) -> Self {
        Self {
            reason,
            code: None,
            unresolved: false,
        }
    }

    /// Whether trying the same send again can only fail the same way (the table above).
    pub(in crate::notifications) const fn is_permanent(self) -> bool {
        if self.unresolved {
            return false;
        }
        match (self.reason, self.code) {
            (
                DeliveryReason::TargetNotPermitted
                | DeliveryReason::BadAddress
                | DeliveryReason::SecretUnreadable
                | DeliveryReason::SetupFailed,
                _,
            ) => true,
            (DeliveryReason::HttpStatus, Some(status)) => {
                matches!(status, 400..=499) && !matches!(status, 408 | 429)
            }
            (DeliveryReason::SmtpRefused, Some(reply)) => matches!(reply, 500..=599),
            (
                DeliveryReason::HttpStatus
                | DeliveryReason::SmtpRefused
                | DeliveryReason::ConnectionFailed
                | DeliveryReason::TlsFailed
                | DeliveryReason::TimedOut
                | DeliveryReason::NoStarttls
                | DeliveryReason::NotUnderstood,
                _,
            ) => false,
        }
    }
}

impl From<SendError> for Failure {
    /// Every egress failure has exactly one reason; only a status or a reply carries a code.
    fn from(error: SendError) -> Self {
        let (reason, code) = match error {
            SendError::Target(_) => (DeliveryReason::TargetNotPermitted, None),
            SendError::Url => (DeliveryReason::BadAddress, None),
            SendError::Setup => (DeliveryReason::SetupFailed, None),
            SendError::Connect => (DeliveryReason::ConnectionFailed, None),
            SendError::Tls => (DeliveryReason::TlsFailed, None),
            SendError::TimedOut => (DeliveryReason::TimedOut, None),
            SendError::HttpStatus(status) => (DeliveryReason::HttpStatus, Some(i32::from(status))),
            SendError::NoStartTls => (DeliveryReason::NoStarttls, None),
            SendError::SmtpReply(reply) => (DeliveryReason::SmtpRefused, Some(i32::from(reply))),
            SendError::Exchange => (DeliveryReason::NotUnderstood, None),
        };
        let unresolved = matches!(error, SendError::Target(TargetError::Unresolved));
        Self {
            reason,
            code,
            unresolved,
        }
    }
}

/// The delivery log entry of one send of `kind` through `channel` that started `at`: ok, or the
/// failure's reason and code. It keeps the channel's name, so it outlives the channel.
pub(in crate::notifications) fn delivery(
    id: DeliveryId,
    channel: &Channel,
    (kind, at): (NotificationKind, DateTime<Utc>),
    sent: Result<(), Failure>,
) -> Delivery {
    let failure = sent.err();
    Delivery {
        id,
        channel: Some(channel.id),
        channel_name: channel.fields.name.clone(),
        kind,
        at,
        ok: failure.is_none(),
        reason: failure.map(|failure| failure.reason),
        code: failure.and_then(|failure| failure.code),
    }
}
