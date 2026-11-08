//! Domain types of web interfaces (WEB-001, WEB-002, WEB-008).

use std::fmt;

use chrono::{DateTime, Utc};

use crate::{
    fingerprint::CertificateFingerprint,
    ids::{CheckId, DeviceId, OperatorId, WebInterfaceId},
};

pub use crate::api_errors::FieldError;

/// Where the client opens an interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenIn {
    AnimaTab,
    SystemBrowser,
}

impl OpenIn {
    /// The `lower_snake` text stored in `interfaces.open_in`.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::AnimaTab => "anima_tab",
            Self::SystemBrowser => "system_browser",
        }
    }

    pub(super) fn from_stored(text: &str) -> Option<Self> {
        match text {
            "anima_tab" => Some(Self::AnimaTab),
            "system_browser" => Some(Self::SystemBrowser),
            _ => None,
        }
    }
}

/// The certificate the operator accepted for an interface, and when. It is stored and shown
/// as 64 lower-case hex digits (D-5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub fingerprint: CertificateFingerprint,
    pub at: DateTime<Utc>,
}

/// A web address kept for a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebInterface {
    pub id: WebInterfaceId,
    pub device: DeviceId,
    pub label: String,
    pub url: String,
    pub open_in: OpenIn,
    pub pin: Option<Pin>,
}

/// An HTTP check's address offered as an interface (WEB-002).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub check: CheckId,
    pub url: String,
    pub label: String,
}

/// What the operator typed for an interface. The label defaults to the URL's host.
#[derive(Clone, Debug)]
pub struct WebInterfaceInput {
    pub label: Option<String>,
    pub url: String,
    pub open_in: OpenIn,
}

/// The operator's accept of a certificate: the fingerprint as typed, who, and when.
#[derive(Clone, Copy, Debug)]
pub struct Acceptance<'a> {
    pub fingerprint: &'a str,
    pub operator: OperatorId,
    pub at: DateTime<Utc>,
}

/// An interface before and after an update, for the audit entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updated {
    pub before: WebInterface,
    pub after: WebInterface,
}

/// The result of accepting a suggestion: the interface, and whether this call made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted {
    pub interface: WebInterface,
    pub created: bool,
}

/// What a lookup could not find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Interface(WebInterfaceId),
    Device(DeviceId),
    Check(CheckId),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interface(id) => write!(f, "web interface {id}"),
            Self::Device(id) => write!(f, "device {id}"),
            Self::Check(id) => write!(f, "check {id}"),
        }
    }
}
