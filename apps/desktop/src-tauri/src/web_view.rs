//! Device web views (WEB-004–008): child web views of the main window, one store per device,
//! no IPC, navigation only to `http` and `https`, their certificate pins, the macOS trust
//! hook that enforces them and the leaf-certificate probe for the system-browser mode (D-9).
//!
//! URLs, hosts, fingerprints and page content are never logged and never part of an error
//! (LOG-007): every error crosses to the client as `{ code }` only.

mod address;
mod bounds;
/// The commands, by path for `generate_handler!`, which needs each command's own module.
pub mod commands;
mod event;
pub use event::MAIN;
mod history;
/// Public only so the pinning proof (`tests/pinning`) can reach `macos_trust::web_view`.
#[cfg(target_os = "macos")]
pub mod macos_trust;
mod opening;
mod storage;
mod tabs;
mod trust;
mod view;

#[cfg(test)]
mod tests;

use serde::{Serialize, Serializer, ser::SerializeStruct};

pub use address::WebUrl;
pub use commands::caller;
pub use tabs::Tabs;
pub use trust::{
    CertificateError, Fingerprint, FingerprintError, OriginError, Pin, PinnedOrigin, Verdict,
    leaf_fingerprint,
};

/// Whether a device web view can pin a certificate on this platform (D-6, D-7).
///
/// The one place that answers it. `true` on macOS, where the trust hook (`macos_trust`,
/// R-069) answers every server-trust challenge with the pin's verdict; `false` elsewhere
/// until the operator has run the OD-7 spike on Linux. While it is `false`, `https` URLs open
/// in the system browser and only `http` is embedded.
pub const fn pinning_available() -> bool {
    cfg!(target_os = "macos")
}

/// Why a web view command was refused. Carries no URL, host, ID or page text (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WebViewError {
    #[error("only the main window may drive device web views")]
    Caller,
    #[error("the tab ID is not 1 to 128 letters, digits, `_` or `-`")]
    TabId,
    #[error("the device ID is not a UUID")]
    DeviceId,
    #[error("the address is not a URL of at most 8192 characters")]
    Url,
    #[error("the address is not http or https")]
    Scheme,
    #[error("the address holds a user name or password")]
    Userinfo,
    #[error("the address has no host")]
    Host,
    #[error("the arguments are not the command's fields")]
    Arguments,
    #[error("the bounds are not finite, non-negative CSS pixels")]
    Bounds,
    #[error("the history step is not BACK, FORWARD or RELOAD")]
    Step,
    #[error("the pin is not a SHA-256 fingerprint")]
    Fingerprint,
    #[error("a pin needs an https address")]
    PinScheme,
    #[error("this platform cannot pin a certificate; use the system browser")]
    PinningUnavailable,
    #[error("a web view is already open for this tab")]
    TabOpen,
    #[error("too many web views are open")]
    TabLimit,
    #[error("no web view is open for this tab")]
    UnknownTab,
    #[error("the main window is not there")]
    Window,
    #[cfg_attr(
        all(target_os = "macos", not(test)),
        expect(dead_code, reason = "only the Linux store folder can fail")
    )]
    #[error("the device's web store could not be placed")]
    Store,
    #[error("the web view did not do what was asked")]
    Native,
    #[error("the address has no pinnable origin")]
    Origin(OriginError),
    #[error("the certificate could not be read")]
    Certificate(CertificateError),
}

impl WebViewError {
    /// Stable code the client switches on (`^[a-z][a-z0-9_]*$`).
    pub fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::TabId => "tab_id",
            Self::DeviceId => "device_id",
            Self::Url => "url",
            Self::Scheme => "scheme",
            Self::Userinfo => "userinfo",
            Self::Host => "host",
            Self::Arguments => "arguments",
            Self::Bounds => "bounds",
            Self::Step => "step",
            Self::Fingerprint => FingerprintError::CODE,
            Self::PinScheme => "pin_scheme",
            Self::PinningUnavailable => "pinning_unavailable",
            Self::TabOpen => "tab_open",
            Self::TabLimit => "tab_limit",
            Self::UnknownTab => "unknown_tab",
            Self::Window => "window",
            Self::Store => "store",
            Self::Native => "native",
            Self::Origin(error) => error.code(),
            Self::Certificate(error) => error.code(),
        }
    }
}

impl From<OriginError> for WebViewError {
    fn from(error: OriginError) -> Self {
        Self::Origin(error)
    }
}

impl From<FingerprintError> for WebViewError {
    fn from(_: FingerprintError) -> Self {
        Self::Fingerprint
    }
}

/// Serialises as `{ "code": ... }` and nothing else (WP-31 "Shell interface").
impl Serialize for WebViewError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("WebViewError", 1)?;
        out.serialize_field("code", self.code())?;
        out.end()
    }
}
