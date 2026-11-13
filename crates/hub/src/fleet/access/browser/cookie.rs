//! Reading one cookie from a request's `Cookie` headers (RFC 6265 Section 5.4). Pure.

use axum::http::{HeaderMap, header::COOKIE};

/// The value of the one cookie called `name` across every `Cookie` header (HTTP/2 may split
/// them), without the double quotes RFC 6265 allows around it.
///
/// `None` when there is no such cookie, more than one (whatever their values), an empty value,
/// or a `Cookie` header that is not text: a header that cannot be read might hide a second
/// cookie of that name, so the answer fails closed.
pub(super) fn single<'h>(headers: &'h HeaderMap, name: &str) -> Option<&'h str> {
    let mut found = None;
    for header in headers.get_all(COOKIE) {
        for pair in header.to_str().ok()?.split(';') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.trim() != name {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(unquoted(value.trim()));
        }
    }
    found.filter(|value| !value.is_empty())
}

/// The value inside one pair of double quotes, or the value itself.
fn unquoted(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

    use super::single;

    const NAME: &str = "__Host-anima";

    fn headers(values: &[&[u8]]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(COOKIE, HeaderValue::from_bytes(value).unwrap());
        }
        headers
    }

    fn read(values: &[&[u8]]) -> Option<String> {
        single(&headers(values), NAME).map(str::to_owned)
    }

    #[test]
    fn d_3_the_one_cookie_is_read_among_others_and_spaces() {
        assert_eq!(read(&[b"__Host-anima=abc"]).as_deref(), Some("abc"));
        assert_eq!(
            read(&[b"theme=dark; __Host-anima=abc;  other=1"]).as_deref(),
            Some("abc")
        );
        assert_eq!(read(&[b"a=1;__Host-anima=abc;b=2"]).as_deref(), Some("abc"));
        assert_eq!(read(&[b" __Host-anima = abc "]).as_deref(), Some("abc"));
        assert_eq!(
            read(&[b"theme=dark", b"__Host-anima=abc"]).as_deref(),
            Some("abc"),
            "split over two headers"
        );
    }

    #[test]
    fn d_3_a_quoted_value_loses_its_quotes() {
        assert_eq!(read(&[br#"__Host-anima="abc""#]).as_deref(), Some("abc"));
        assert_eq!(read(&[br#"__Host-anima="abc"#]).as_deref(), Some(r#""abc"#));
        assert_eq!(read(&[br#"__Host-anima="""#]), None, "quoted empty");
    }

    #[test]
    fn a_3_two_cookies_of_the_name_are_none() {
        assert_eq!(read(&[b"__Host-anima=abc; __Host-anima=abc"]), None);
        assert_eq!(read(&[b"__Host-anima=abc", b"__Host-anima=def"]), None);
        assert_eq!(read(&[b"__Host-anima=; __Host-anima=abc"]), None);
        assert_eq!(
            read(&[b"__Host-anima=abc; __Host-anima"]),
            Some("abc".into())
        );
    }

    #[test]
    fn a_3_other_names_and_empty_values_are_none() {
        assert_eq!(read(&[]), None);
        assert_eq!(read(&[b""]), None);
        assert_eq!(read(&[b"__Host-anima="]), None);
        assert_eq!(read(&[b"__Host-anima"]), None);
        assert_eq!(
            read(&[b"__host-anima=abc"]),
            None,
            "names are case-sensitive"
        );
        assert_eq!(read(&[b"__Secure-anima=abc; anima=abc"]), None);
        assert_eq!(read(&[b"x__Host-anima=abc"]), None);
    }

    #[test]
    fn a_3_a_cookie_header_that_is_not_text_fails_closed() {
        assert_eq!(read(&[b"__Host-anima=abc", b"x=\xff"]), None);
        assert_eq!(read(&[b"x=\xff; __Host-anima=abc"]), None);
    }
}
