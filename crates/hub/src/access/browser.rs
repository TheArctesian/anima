//! Browser sign-in at the HTTP edge: which requests may use the `__Host-anima` cookie, and the
//! `Set-Cookie` values that give and take it (D-3, D-4, WP-43 A-3, A-4, A-8).

mod cookie;

use axum::http::{
    HeaderMap, HeaderValue,
    header::{AUTHORIZATION, ORIGIN},
};
use zeroize::Zeroizing;

use super::{
    PublicOrigin,
    tokens::{BROWSER_LIFETIME, Token},
};

/// The cookie that holds a browser token. `__Host-` makes the browser keep it only when it is
/// `Secure`, has `Path=/` and no `Domain`, so no other host under the domain can set it.
const COOKIE_NAME: &str = "__Host-anima";

/// What every `Set-Cookie` of [`COOKIE_NAME`] carries besides its value and `Max-Age`.
const ATTRIBUTES: &str = "Path=/; Secure; HttpOnly; SameSite=Strict";

/// The mark of a request whose one `Origin` header is the public origin, byte for byte (A-4).
///
/// Only [`SameOrigin::of`] makes one, after that check. `SameSite=Strict` does not stop a page on
/// another host of the same site, such as a device under `net.anima.ing`, from sending the
/// cookie; the Origin check does (D-3).
pub struct SameOrigin(());

impl SameOrigin {
    /// The mark, when `headers` hold exactly one `Origin` header equal to `origin`. `None` for no
    /// header, two, `null`, or any other spelling.
    #[must_use]
    pub fn of(headers: &HeaderMap, origin: &PublicOrigin) -> Option<Self> {
        let mut values = headers.get_all(ORIGIN).iter();
        let only = values.next()?;
        let same = values.next().is_none() && only.as_bytes() == origin.as_str().as_bytes();
        same.then_some(Self(()))
    }
}

/// The browser token in the `__Host-anima` cookie of a request from the public origin: `None`
/// when the request has any `Authorization` header (then only the bearer counts) or not exactly
/// one non-empty cookie of that name (A-3, K7).
pub(super) fn cookie_token<'h>(headers: &'h HeaderMap, _from: &SameOrigin) -> Option<&'h str> {
    if headers.contains_key(AUTHORIZATION) {
        return None;
    }
    cookie::single(headers, COOKIE_NAME)
}

/// The `Set-Cookie` value that gives the browser `token` for 7 days. Marked sensitive, so its
/// `Debug` shows no token (LOG-007).
pub(super) fn signed_in(token: &Token) -> HeaderValue {
    let text = Zeroizing::new(format!(
        "{COOKIE_NAME}={}; Max-Age={}; {ATTRIBUTES}",
        token.expose(),
        BROWSER_LIFETIME.num_seconds()
    ));
    let mut value = HeaderValue::from_str(&text).expect("a token is base64url text");
    value.set_sensitive(true);
    value
}

/// The `Set-Cookie` value that makes the browser drop the cookie at once (A-8).
pub(super) fn signed_out() -> HeaderValue {
    HeaderValue::from_static("__Host-anima=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Strict")
}

#[cfg(test)]
mod tests;
