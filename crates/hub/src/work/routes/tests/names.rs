//! The rules of a public name (Section 12.3 and 12.4; SF-NEW-routetoken, SF-NEW-routeabuse).

use super::support::{HUB, name, rules};
use crate::work::routes::names::{NameRefusal, RouteName};

fn refusal(text: &str) -> NameRefusal {
    RouteName::parse(text, &rules()).unwrap_err()
}

#[test]
fn a_name_is_kept_in_lower_case_without_its_final_dot() {
    assert_eq!(name(" Photos.Example.COM. ").as_str(), "photos.example.com");
}

#[test]
fn a_wildcard_is_refused() {
    assert_eq!(refusal("*.example.com"), NameRefusal::Wildcard);
}

#[test]
fn the_name_of_the_hub_and_each_name_below_it_are_refused() {
    assert_eq!(refusal(HUB), NameRefusal::Hub);
    assert_eq!(refusal("app.hub.anima.ing"), NameRefusal::Hub);
}

#[test]
fn a_name_at_or_below_a_cookie_domain_is_refused() {
    assert_eq!(refusal("anima.ing"), NameRefusal::CookieDomain);
    assert_eq!(refusal("photos.anima.ing"), NameRefusal::CookieDomain);
    // A different domain that only ends with the same letters is not below it.
    assert_eq!(name("notanima.ing").as_str(), "notanima.ing");
}

#[test]
fn an_address_a_path_or_a_port_is_not_a_name() {
    for text in [
        "10.0.0.1",
        "example.com/app",
        "example.com:8443",
        "user@example.com",
    ] {
        assert_eq!(refusal(text), NameRefusal::NotAHost, "{text}");
    }
}

#[test]
fn a_bad_form_or_length_is_refused() {
    for text in [
        "localhost",
        "-a.example.com",
        "a-.example.com",
        "a..example.com",
        "a_b.example.com",
    ] {
        assert_eq!(refusal(text), NameRefusal::Form, "{text}");
    }
    assert_eq!(refusal(""), NameRefusal::Length);
    let long = format!("{}.com", "a.".repeat(130));
    assert_eq!(refusal(&long), NameRefusal::Length);
    let label = format!("{}.com", "a".repeat(64));
    assert_eq!(refusal(&label), NameRefusal::Form);
}

#[test]
fn a_name_beside_the_hub_is_same_site_and_a_different_domain_is_not() {
    let hub = "hub.example.com";
    assert!(name("photos.example.com").same_site_as(hub));
    assert!(!name("photos.example.org").same_site_as(hub));
    assert!(!name("photos.example.com").same_site_as("example.com"));
}

#[test]
fn caa_is_read_from_the_name_up_to_the_registered_domain() {
    assert_eq!(
        name("a.b.example.com").with_parents(),
        ["a.b.example.com", "b.example.com", "example.com"]
    );
}
