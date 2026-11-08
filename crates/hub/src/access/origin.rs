//! The hub's public origin, from `ANIMA_PUBLIC_ORIGIN` (K3, D-3): the web client's
//! `connect-src` uses it.

/// The canonical origin text `scheme://host[:port]`, as config validated it: `https`, or
/// `http` for a loopback host only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicOrigin(String);

impl PublicOrigin {
    /// Wraps origin text that config has already made canonical.
    #[must_use]
    pub const fn new(canonical: String) -> Self {
        Self(canonical)
    }

    /// The canonical text, for example `https://hub.anima.ing`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
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
