//! The Origin mark and the cookie's `Set-Cookie` values (D-3, A-3, A-4, A-8).

use axum::http::{
    HeaderMap, HeaderValue,
    header::{AUTHORIZATION, COOKIE, ORIGIN},
};

use super::{SameOrigin, cookie_token, signed_in, signed_out};
use crate::fleet::access::{PublicOrigin, tokens::Token};

const PUBLIC: &str = "https://hub.anima.ing";

fn origin() -> PublicOrigin {
    PublicOrigin::new(PUBLIC.to_owned())
}

fn with(entries: &[(axum::http::HeaderName, &[u8])]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in entries {
        headers.append(name, HeaderValue::from_bytes(value).unwrap());
    }
    headers
}

fn marked(headers: &HeaderMap) -> bool {
    SameOrigin::of(headers, &origin()).is_some()
}

#[test]
fn a_4_only_one_origin_header_equal_byte_for_byte_marks_the_request() {
    assert!(marked(&with(&[(ORIGIN, PUBLIC.as_bytes())])));
    for other in [
        &b"https://hub.anima.ing/"[..],
        b"https://hub.anima.ing:443",
        b"HTTPS://hub.anima.ing",
        b"https://HUB.anima.ing",
        b"http://hub.anima.ing",
        b"https://hub.anima.ing.evil.example",
        b"https://device.net.anima.ing",
        b"https://hub.anima.in",
        b" https://hub.anima.ing",
        b"null",
        b"",
        b"tauri://localhost",
    ] {
        let headers = with(&[(ORIGIN, other)]);
        assert!(!marked(&headers), "{}", String::from_utf8_lossy(other));
    }
    assert!(!marked(&HeaderMap::new()), "no Origin");
    let twice = with(&[(ORIGIN, PUBLIC.as_bytes()), (ORIGIN, PUBLIC.as_bytes())]);
    assert!(!marked(&twice), "two Origin headers");
}

#[test]
fn a_4_an_origin_that_is_not_http_text_marks_no_request() {
    for text in ["null", "", "https://", "tauri://localhost"] {
        let origin = PublicOrigin::new(text.to_owned());
        let headers = with(&[(ORIGIN, text.as_bytes())]);
        assert!(SameOrigin::of(&headers, &origin).is_none(), "{text:?}");
    }
}

#[test]
fn a_3_the_cookie_counts_only_without_an_authorization_header() {
    let mark = SameOrigin::of(&with(&[(ORIGIN, PUBLIC.as_bytes())]), &origin()).unwrap();
    let cookie = with(&[(COOKIE, b"__Host-anima=abc")]);
    assert_eq!(cookie_token(&cookie, &mark), Some("abc"));
    let both = with(&[
        (COOKIE, b"__Host-anima=abc"),
        (AUTHORIZATION, b"Basic eA=="),
    ]);
    assert_eq!(cookie_token(&both, &mark), None);
    let empty_authorization = with(&[(COOKIE, b"__Host-anima=abc"), (AUTHORIZATION, b"")]);
    assert_eq!(cookie_token(&empty_authorization, &mark), None);
}

#[test]
fn d_3_the_cookie_is_set_with_every_attribute_and_cleared_with_the_same() {
    let token = Token::from_random([3; 32]);
    let set = signed_in(&token);
    assert_eq!(
        set.to_str().unwrap(),
        format!(
            "__Host-anima={}; Max-Age=604800; Path=/; Secure; HttpOnly; SameSite=Strict",
            token.expose()
        )
    );
    assert!(set.is_sensitive());
    assert!(!format!("{set:?}").contains(token.expose()), "LOG-007");
    assert_eq!(
        signed_out().to_str().unwrap(),
        "__Host-anima=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Strict"
    );
}
