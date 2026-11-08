//! The hub's public origin, `ANIMA_PUBLIC_ORIGIN`: the address clients use, which the cookie
//! Origin check, share links and the web client's CSP name (D-3, K7).

/// The public origin as canonical text, `scheme://host[:port]`; the configuration checks the
/// spelling before it builds one.
#[derive(Clone)]
pub struct PublicOrigin(String);

impl PublicOrigin {
    /// The origin `canonical`, already checked by the configuration.
    #[must_use]
    pub fn new(canonical: String) -> Self {
        Self(canonical)
    }

    /// The origin as a browser sends it in an `Origin` header.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The WebSocket origin of the same host and port: `wss://` for `https://`, `ws://` for a
    /// plain `http://` origin.
    #[must_use]
    pub fn websocket(&self) -> String {
        self.0
            .strip_prefix("http")
            .map_or_else(|| self.0.clone(), |rest| format!("ws{rest}"))
    }
}
