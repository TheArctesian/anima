//! The addresses a device web view may load (D-3, D-6, WEB-007): only `http` and `https`, and
//! `https` only where the platform can pin a certificate. Pure: no I/O.

use tauri::Url;

use super::{Fingerprint, Pin, PinnedOrigin, WebViewError};

/// The longest address accepted, as the client's own bound.
const URL_MAX: usize = 8192;

/// Whether `url` is a web address: `http` or `https`, nothing else.
pub fn is_web(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// Whether a device web view may go to `url`: `http`, or `https` where the platform can pin
/// (D-6). The navigation filter, asked for every frame and every new-window request, so a
/// page cannot take its view to an `https` address the view could not pin.
pub fn may_load(url: &Url, pinning: bool) -> bool {
    is_web(url) && (url.scheme() == "http" || pinning)
}

/// An `http` or `https` address with a host and no user name or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebUrl(Url);

impl WebUrl {
    /// Checks an address from the client. The error names the rule broken, never the text.
    pub fn parse(text: &str) -> Result<Self, WebViewError> {
        if text.len() > URL_MAX {
            return Err(WebViewError::Url);
        }
        let url = Url::parse(text).map_err(|_| WebViewError::Url)?;
        if !is_web(&url) {
            return Err(WebViewError::Scheme);
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(WebViewError::Userinfo);
        }
        if url.host_str().is_none_or(str::is_empty) {
            return Err(WebViewError::Host);
        }
        Ok(Self(url))
    }

    pub fn is_https(&self) -> bool {
        self.0.scheme() == "https"
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }

    /// The normalised address, as the URL parser writes it.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// How a web view opens an address, by the D-6 rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Embedding {
    /// `http`: no certificate, nothing to pin.
    Plain,
    /// `https` on a platform that pins: the origin a pin belongs to, and the pin if the
    /// operator has accepted one.
    Pinned {
        origin: PinnedOrigin,
        pin: Option<Pin>,
    },
}

/// Decides whether `url` may be embedded. `https` needs a platform that pins (otherwise the
/// client opens the system browser); a pin needs `https`.
pub fn embedding(
    url: &WebUrl,
    pin: Option<Fingerprint>,
    pinning: bool,
) -> Result<Embedding, WebViewError> {
    if !url.is_https() {
        return match pin {
            Some(_) => Err(WebViewError::PinScheme),
            None => Ok(Embedding::Plain),
        };
    }
    if !pinning {
        return Err(WebViewError::PinningUnavailable);
    }
    let origin = PinnedOrigin::parse(url.as_str())?;
    let pin = pin.map(|fingerprint| Pin {
        origin: origin.clone(),
        fingerprint,
    });
    Ok(Embedding::Pinned { origin, pin })
}
