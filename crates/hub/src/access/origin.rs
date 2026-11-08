//! The hub's public origin, `ANIMA_PUBLIC_ORIGIN`: the address clients use, which the cookie
//! Origin check, share links and the web client's CSP name (D-3, K7).

/// The public origin as canonical text, `scheme://host[:port]`; the configuration checks the
/// spelling before it builds one.
#[derive(Clone)]
pub struct PublicOrigin(String);

impl PublicOrigin {
    /// The origin `canonical`, already checked by the configuration. Text that is not
    /// `http://` or `https://` and a host, such as `null` or empty text, never matches a
    /// browser's `Origin` header, so such an origin marks no request (D-3).
    #[must_use]
    pub const fn new(canonical: String) -> Self {
        Self(canonical)
    }

    /// The origin's text, for example `https://hub.anima.ing`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `Origin` header a browser at this origin sends; `None` unless the text is
    /// `http://` or `https://` and a host, so the check fails closed on any other text.
    pub(super) fn origin_header(&self) -> Option<&str> {
        let authority = self
            .0
            .strip_prefix("https://")
            .or_else(|| self.0.strip_prefix("http://"))?;
        (!authority.is_empty()).then_some(self.0.as_str())
    }

    /// The WebSocket origin of the same host and port: `wss://host[:port]` for an `https`
    /// origin, `ws://host[:port]` for an `http` one. Empty for any other scheme, which config
    /// never lets through.
    #[must_use]
    pub fn websocket(&self) -> String {
        if let Some(authority) = self.0.strip_prefix("https://") {
            format!("wss://{authority}")
        } else if let Some(authority) = self.0.strip_prefix("http://") {
            format!("ws://{authority}")
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests;
