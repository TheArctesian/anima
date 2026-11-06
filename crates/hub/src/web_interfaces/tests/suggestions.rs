//! WEB-002 and D-7: which HTTP checks are offered.

use std::collections::HashSet;

use crate::{
    ids::{CheckId, DeviceId, WebInterfaceId},
    web_interfaces::{OpenIn, WebInterface, suggestion::filter},
};

fn check(n: u128) -> CheckId {
    CheckId::from(uuid::Uuid::from_u128(n))
}

fn interface(url: &str) -> WebInterface {
    WebInterface {
        id: WebInterfaceId::from(uuid::Uuid::from_u128(1)),
        device: DeviceId::from(uuid::Uuid::from_u128(2)),
        label: "x".to_owned(),
        url: url.to_owned(),
        open_in: OpenIn::AnimaTab,
        pin: None,
    }
}

fn urls(list: &[(u128, &str)]) -> Vec<(CheckId, String)> {
    list.iter()
        .map(|(n, u)| (check(*n), (*u).to_owned()))
        .collect()
}

#[test]
fn d_7_offers_each_http_check_labelled_with_its_host() {
    let offered = filter(
        &urls(&[(1, "http://ha.home.lan:8123/"), (2, "https://nas.lan/ui")]),
        &[],
        &HashSet::new(),
    );
    let seen: Vec<_> = offered
        .iter()
        .map(|s| (s.check, s.label.as_str()))
        .collect();
    assert_eq!(seen, [(check(1), "ha.home.lan"), (check(2), "nas.lan")]);
    assert_eq!(offered[0].url, "http://ha.home.lan:8123/");
}

#[test]
fn d_7_hides_a_dismissed_check() {
    let dismissed = HashSet::from([check(1)]);
    let offered = filter(
        &urls(&[(1, "http://a.lan/"), (2, "http://b.lan/")]),
        &[],
        &dismissed,
    );
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].check, check(2));
}

#[test]
fn d_7_hides_a_url_that_is_already_an_interface_compared_as_parsed_urls() {
    let have = [interface("HTTP://Ha.Home.Lan:80")];
    let offered = filter(
        &urls(&[(1, "http://ha.home.lan/"), (2, "http://ha.home.lan/other")]),
        &have,
        &HashSet::new(),
    );
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].check, check(2));
}

#[test]
fn log_007_never_offers_a_check_url_with_a_user_name_or_one_the_rule_refuses() {
    let offered = filter(
        &urls(&[
            (1, "http://admin:secret@a.lan/"),
            (2, "http://a.lan/#x"),
            (3, "not a url"),
        ]),
        &[],
        &HashSet::new(),
    );
    assert!(offered.is_empty());
}
