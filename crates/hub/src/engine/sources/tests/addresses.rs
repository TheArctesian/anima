//! K-E1 Section 7: the URL is always a declared API base plus the request's path; any path that
//! could leave the base, and any base that is not declared, is refused before a send.

use proptest::prelude::*;
use reqwest::Url;
use source_exchange::{Method, Request};

use crate::{
    engine::sources::acquisition::transport::{Fault, Transport, address},
    fleet::targets::AddressPolicy,
};

const BASE: &str = "https://api.github.com";

fn request(path: &str) -> Request {
    Request {
        method: Method::Get,
        path: path.to_owned(),
        query: vec![("per_page".to_owned(), "100".to_owned())],
    }
}

#[test]
fn src_006_a_plain_path_is_appended_to_the_base_with_its_query() {
    let base = Url::parse(BASE).unwrap();
    let url = address(&base, "/installation/repositories", &request("/").query).unwrap();
    assert_eq!(
        url.as_str(),
        "https://api.github.com/installation/repositories?per_page=100"
    );
    let nested = Url::parse("https://example.test/api/v3/").unwrap();
    let url = address(&nested, "/repos/a/b", &[]).unwrap();
    assert_eq!(url.as_str(), "https://example.test/api/v3/repos/a/b");
}

#[test]
fn src_006_a_path_that_could_leave_the_base_is_refused() {
    let base = Url::parse(BASE).unwrap();
    for path in [
        "repos",
        "//evil.example/x",
        "/https://evil.example/x",
        "/x?u=1",
        "/x#frag",
        "/user@evil.example",
        "/a\\b",
        "/a b",
        "/a\nb",
        "",
    ] {
        assert_eq!(address(&base, path, &[]), Err(Fault::Refused), "{path:?}");
    }
}

#[test]
fn src_008_a_dot_segment_or_an_encoded_dot_is_refused() {
    let base = Url::parse(BASE).unwrap();
    for path in [
        "/repos/o/n/git/trees/../../../../../app/installations",
        "/repos/o/n/git/trees/main/..",
        "/repos/./x",
        "/.",
        "/repos/o/n/git/trees/%2e%2e/%2E%2E/app",
        "/repos/%2E./x",
        "/repos/.%2e/x",
        "/repos/a%2eb",
    ] {
        assert_eq!(address(&base, path, &[]), Err(Fault::Refused), "{path:?}");
    }
    assert!(
        address(&base, "/repos/o/n.js/git/trees/v1.2", &[]).is_ok(),
        "a dot inside a segment is plain"
    );
}

#[test]
fn src_008_a_base_with_a_path_cannot_be_escaped() {
    let base = Url::parse("https://example.test/api/v3").unwrap();
    for path in ["/../app/installations", "/x/../../app", "/%2e%2e/app"] {
        assert_eq!(address(&base, path, &[]), Err(Fault::Refused), "{path:?}");
    }
    let url = address(&base, "/repos/a/b", &[]).unwrap();
    assert_eq!(url.path(), "/api/v3/repos/a/b");
}

#[test]
fn src_006_only_a_declared_https_base_is_a_destination() {
    let transport = Transport::new(
        [BASE, "https://10.0.0.1", "http://plain.example"],
        AddressPolicy::default(),
    )
    .unwrap();
    assert!(transport.url(BASE, &request("/x")).is_ok());
    assert!(
        transport
            .url("https://API.GitHub.com", &request("/x"))
            .is_ok(),
        "names compare in any case"
    );
    for base in [
        "https://evil.example",
        "https://10.0.0.1",
        "http://plain.example",
        "http://api.github.com",
    ] {
        assert_eq!(
            transport.url(base, &request("/x")),
            Err(Fault::Refused),
            "{base}"
        );
    }
}

proptest! {
    /// Whatever the path, a URL that is made keeps the base's scheme, host and port, and starts
    /// with the base's path.
    #[test]
    fn src_006_no_path_moves_the_url_off_its_base(path in "\\PC{0,40}", raw in proptest::collection::vec(any::<u8>(), 0..40)) {
        let base = Url::parse("https://api.github.com:443/v3").unwrap();
        let lossy = String::from_utf8_lossy(&raw).into_owned();
        for candidate in [path, lossy, format!("/{}", "x")] {
            if let Ok(url) = address(&base, &candidate, &[]) {
                prop_assert_eq!(url.scheme(), "https");
                prop_assert_eq!(url.host_str(), Some("api.github.com"));
                prop_assert_eq!(url.port_or_known_default(), Some(443));
                prop_assert!(url.path().starts_with("/v3/"));
                prop_assert!(url.username().is_empty());
            }
        }
    }

    /// Whatever mix of plain, dot and encoded-dot segments, a URL that is made stays below the
    /// base's path.
    #[test]
    fn src_008_no_mix_of_dot_segments_leaves_the_base_path(
        segments in proptest::collection::vec(
            prop_oneof![
                Just(".".to_owned()),
                Just("..".to_owned()),
                Just("%2e".to_owned()),
                Just("%2E%2e".to_owned()),
                Just(".%2E".to_owned()),
                "[a-z.]{1,4}",
            ],
            1..8,
        )
    ) {
        let base = Url::parse("https://example.test/api/v3").unwrap();
        let path = format!("/{}", segments.join("/"));
        if let Ok(url) = address(&base, &path, &[]) {
            prop_assert!(url.path().starts_with("/api/v3/"), "{}", url.path());
            prop_assert!(!url.path().split('/').any(|s| s == "." || s == ".."));
        }
    }
}
