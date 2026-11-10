//! The IANA bootstrap file (D-29, RFC 9224): parsing, the suffix match and its refusals.

use proptest::prelude::*;

use crate::checks::probe::{
    outcome::Failure,
    rdap::bootstrap::{Bootstrap, Unreadable},
};

/// A file shaped as IANA publishes it, with the entries the tests need.
const FILE: &str = r#"{
  "version": "1.0",
  "publication": "2026-11-01T00:00:00Z",
  "description": "RDAP bootstrap file for Domain Name System registrations",
  "services": [
    [["com", "net"], ["https://rdap.verisign.example/com/v1/", "http://rdap.verisign.example/com/v1/"]],
    [["uk"], ["https://rdap.nominet.example/uk"]],
    [["co.uk"], ["https://rdap.second.example/"]],
    [["XN--P1AI"], ["https://rdap.rf.example/"]],
    [["plain"], ["http://rdap.plain.example/"]],
    [["user"], ["https://name:secret@rdap.user.example/"]],
    "not a service",
    [["broken"], [7]],
    [[], ["https://rdap.nothing.example/"]]
  ]
}"#;

fn file() -> Bootstrap {
    Bootstrap::parse(FILE.as_bytes()).unwrap()
}

fn query(domain: &str) -> Result<String, Failure> {
    file().query(domain).map(String::from)
}

#[test]
fn d_29_a_domain_is_asked_at_the_https_base_of_its_tld() {
    assert_eq!(
        query("example.net").unwrap(),
        "https://rdap.verisign.example/com/v1/domain/example.net"
    );
    assert_eq!(
        query("www.example.com").unwrap(),
        "https://rdap.verisign.example/com/v1/domain/www.example.com"
    );
    // A base URL without a trailing slash still keeps its path.
    assert_eq!(
        query("example.uk").unwrap(),
        "https://rdap.nominet.example/uk/domain/example.uk"
    );
    // Suffixes compare without regard to case; IDN TLDs come in their A-label form.
    assert_eq!(
        query("example.xn--p1ai").unwrap(),
        "https://rdap.rf.example/domain/example.xn--p1ai"
    );
}

#[test]
fn d_29_the_suffix_with_the_most_labels_wins() {
    assert_eq!(
        query("example.co.uk").unwrap(),
        "https://rdap.second.example/domain/example.co.uk"
    );
    assert_eq!(
        query("example.ac.uk").unwrap(),
        "https://rdap.nominet.example/uk/domain/example.ac.uk"
    );
}

#[test]
fn d_29_a_suffix_matches_whole_labels_only() {
    assert_eq!(query("example.network"), Err(Failure::NoRdapService));
    assert_eq!(query("examplenet"), Err(Failure::NoRdapService));
    assert_eq!(query("example.org"), Err(Failure::NoRdapService));
}

#[test]
fn p_6_a_service_without_an_https_base_gives_no_address() {
    // Neither plain HTTP nor a base URL with credentials is used, and a shorter suffix never
    // stands in for the service that matches.
    assert_eq!(query("example.plain"), Err(Failure::RdapNotHttps));
    assert_eq!(query("example.user"), Err(Failure::RdapNotHttps));
}

#[test]
fn d_29_entries_that_are_not_services_are_skipped() {
    assert_eq!(query("example.broken"), Err(Failure::NoRdapService));
}

#[test]
fn d_29_a_body_that_is_not_the_bootstrap_object_is_unreadable() {
    for body in [
        &b""[..],
        b"null",
        b"[]",
        b"{}",
        b"{\"services\": {}}",
        b"{\"services\": [",
        b"\xff\xfe",
    ] {
        assert_eq!(Bootstrap::parse(body), Err(Unreadable), "{body:?}");
    }
}

proptest! {
    /// P3-003: the bootstrap file comes from the network, so no bytes make the parser or the
    /// match panic, and every address it gives is HTTPS.
    #[test]
    fn p3_003_any_bytes_parse_without_a_panic(body in proptest::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(read) = Bootstrap::parse(&body)
            && let Ok(url) = read.query("example.net")
        {
            prop_assert_eq!(url.scheme(), "https");
        }
    }

    /// P3-003: service lists of any strings give HTTPS addresses only, under the base given.
    #[test]
    fn p3_003_any_services_give_https_addresses_only(
        suffix in "[a-z]{1,4}",
        base in "\\PC{0,40}",
    ) {
        let file = serde_json::json!({ "services": [[[suffix.clone()], [base]]] });
        let read = Bootstrap::parse(file.to_string().as_bytes()).unwrap();
        if let Ok(url) = read.query(&format!("example.{suffix}")) {
            prop_assert_eq!(url.scheme(), "https");
            let path_ends_with_query = url.path().ends_with(&format!("/domain/example.{suffix}"));
            prop_assert!(path_ends_with_query);
        }
    }
}
