//! D-4, WEB-008: every verdict on a server-trust challenge, and the answer the trust hook
//! gives `WebKit` for it (R-069: a mismatched certificate is cancelled).

use std::cell::Cell;

use crate::web_view::{
    Fingerprint, Pin, PinnedOrigin,
    trust::{Answer, Challenge, Presented, Verdict, rule, verdict},
};

fn fingerprint(byte: u8) -> Fingerprint {
    Fingerprint::of_certificate(&[byte])
}

fn pin(url: &str) -> Pin {
    Pin {
        origin: PinnedOrigin::parse(url).unwrap(),
        fingerprint: fingerprint(1),
    }
}

fn challenge(host: &str, port: u16, presented: u8, system_trusted: bool) -> Challenge<'_> {
    Challenge {
        host,
        port,
        presented: fingerprint(presented),
        system_trusted,
    }
}

#[test]
fn d_4_the_pinned_certificate_is_accepted_whatever_the_system_trust() {
    let pin = pin("https://router.lan/");
    for trusted in [false, true] {
        let answer = verdict(Some(&pin), &challenge("router.lan", 443, 1, trusted));
        assert_eq!(answer, Verdict::Accept, "trusted {trusted}");
    }
}

#[test]
fn d_4_another_certificate_on_the_pinned_origin_is_a_mismatch_even_when_trusted() {
    let pin = pin("https://router.lan/");
    for trusted in [false, true] {
        let answer = verdict(Some(&pin), &challenge("router.lan", 443, 2, trusted));
        let expected = Verdict::Mismatch {
            expected: fingerprint(1),
            presented: fingerprint(2),
        };
        assert_eq!(answer, expected, "trusted {trusted}");
    }
}

#[test]
fn d_4_another_host_or_port_gets_the_system_handling() {
    let pin = pin("https://router.lan:8443/");
    let others = [
        ("router.lan", 443),
        ("nas.lan", 8443),
        ("router.la", 8443),
        ("router.lan.lan", 8443),
    ];
    for (host, port) in others {
        for (presented, trusted) in [(1, false), (2, false), (1, true), (2, true)] {
            let answer = verdict(Some(&pin), &challenge(host, port, presented, trusted));
            assert_eq!(
                answer,
                Verdict::Default,
                "{host}:{port} {presented} {trusted}"
            );
        }
    }
}

#[test]
fn d_4_without_a_pin_a_trusted_chain_gets_the_system_handling() {
    let answer = verdict(None, &challenge("router.lan", 443, 2, true));
    assert_eq!(answer, Verdict::Default);
}

#[test]
fn d_4_without_a_pin_an_untrusted_chain_shows_its_fingerprint() {
    let answer = verdict(None, &challenge("router.lan", 443, 2, false));
    assert_eq!(answer, Verdict::Untrusted(fingerprint(2)));
}

#[test]
fn d_4_the_challenge_host_matches_in_any_case_and_with_one_trailing_dot() {
    let pin = pin("https://Router.lan/");
    for host in ["router.lan", "ROUTER.LAN", "router.lan."] {
        let answer = verdict(Some(&pin), &challenge(host, 443, 2, true));
        assert!(matches!(answer, Verdict::Mismatch { .. }), "{host}");
    }
    for host in ["router.lan..", "router.lan ", "xrouter.lan", ""] {
        let answer = verdict(Some(&pin), &challenge(host, 443, 2, true));
        assert_eq!(answer, Verdict::Default, "{host:?}");
    }
}

#[test]
fn d_4_an_address_matches_with_or_without_brackets_in_any_spelling() {
    let v6 = pin("https://[fd00::1]/");
    for host in ["fd00::1", "[fd00::1]", "FD00:0:0::1", "[FD00:0000::0001]"] {
        assert_eq!(
            verdict(Some(&v6), &challenge(host, 443, 1, false)),
            Verdict::Accept,
            "{host}"
        );
    }
    for host in ["fd00::2", "[fd00::1", "fd00::1]"] {
        assert_eq!(
            verdict(Some(&v6), &challenge(host, 443, 1, false)),
            Verdict::Default,
            "{host}"
        );
    }
    let v4 = pin("https://192.168.1.1/");
    assert_eq!(
        verdict(Some(&v4), &challenge("192.168.1.1", 443, 1, false)),
        Verdict::Accept
    );
    assert_eq!(
        verdict(Some(&v4), &challenge("192.168.1.10", 443, 1, false)),
        Verdict::Default
    );
}

/// The answer and shown verdict for a challenge, and how often system trust was asked.
fn answered(
    pin: Option<&Pin>,
    opened_on: &str,
    presented: &Presented<'_>,
    trusted: bool,
) -> (Answer, Option<Verdict>, u32) {
    let asked = Cell::new(0);
    let origin = PinnedOrigin::parse(opened_on).ok();
    let ruling = rule(pin, origin.as_ref(), presented, || {
        asked.set(asked.get() + 1);
        trusted
    });
    (ruling.answer, ruling.shown, asked.get())
}

fn presented(host: &str, port: isize, leaf: u8) -> Presented<'_> {
    Presented {
        host,
        port,
        leaf: Some(fingerprint(leaf)),
    }
}

#[test]
fn r_069_a_mismatched_certificate_is_cancelled_and_shown_whatever_the_system_trust() {
    let pin = pin("https://router.lan/");
    for trusted in [false, true] {
        let answer = answered(
            Some(&pin),
            "https://router.lan/",
            &presented("router.lan", 443, 2),
            trusted,
        );
        let mismatch = Verdict::Mismatch {
            expected: fingerprint(1),
            presented: fingerprint(2),
        };
        assert_eq!(
            answer,
            (Answer::Cancel, Some(mismatch), 0),
            "trusted {trusted}"
        );
    }
}

#[test]
fn web_008_the_pinned_certificate_is_accepted_without_asking_the_system() {
    let pin = pin("https://router.lan/");
    let answer = answered(
        Some(&pin),
        "https://router.lan/",
        &presented("router.lan", 443, 1),
        false,
    );
    assert_eq!(answer, (Answer::Accept, None, 0));
}

#[test]
fn d_4_another_origin_of_a_pinned_tab_gets_the_default_without_asking_the_system() {
    let pin = pin("https://router.lan/");
    let answer = answered(
        Some(&pin),
        "https://router.lan/",
        &presented("cdn.lan", 443, 2),
        false,
    );
    assert_eq!(answer, (Answer::Default, None, 0));
}

#[test]
fn web_008_without_a_pin_a_trusted_chain_gets_the_default() {
    let answer = answered(
        None,
        "https://router.lan/",
        &presented("router.lan", 443, 2),
        true,
    );
    assert_eq!(answer, (Answer::Default, None, 1));
}

#[test]
fn web_008_an_untrusted_chain_on_the_tabs_origin_is_cancelled_and_shown() {
    let answer = answered(
        None,
        "https://router.lan/",
        &presented("ROUTER.lan.", 443, 2),
        false,
    );
    assert_eq!(
        answer,
        (Answer::Cancel, Some(Verdict::Untrusted(fingerprint(2))), 1)
    );
}

#[test]
fn web_008_an_untrusted_chain_elsewhere_is_cancelled_and_not_shown() {
    let elsewhere = [("cdn.lan", 443), ("router.lan", 8443)];
    for opened_on in ["https://router.lan/", "http://router.lan/"] {
        for (host, port) in elsewhere {
            let answer = answered(None, opened_on, &presented(host, port, 2), false);
            assert_eq!(answer.0, Answer::Cancel, "{opened_on} {host}:{port}");
            assert_eq!(answer.1, None, "{opened_on} {host}:{port}");
        }
    }
}

#[test]
fn web_008_no_leaf_no_host_or_no_port_is_cancelled_without_asking_the_system() {
    let pin = pin("https://router.lan/");
    let no_leaf = Presented {
        host: "router.lan",
        port: 443,
        leaf: None,
    };
    let broken = [
        no_leaf,
        presented("router lan", 443, 1),
        presented("", 443, 1),
        presented("router.lan/x", 443, 1),
        presented("router.lan", -1, 1),
        presented("router.lan", 65_536, 1),
    ];
    for challenge in broken {
        for tab_pin in [None, Some(&pin)] {
            let answer = answered(tab_pin, "https://router.lan/", &challenge, true);
            assert_eq!(answer, (Answer::Cancel, None, 0), "{challenge:?}");
        }
    }
}

#[test]
fn d_4_a_challenge_without_a_port_is_for_443_so_the_pin_decides() {
    let pin = pin("https://router.lan/");
    let answer = answered(
        Some(&pin),
        "https://router.lan/",
        &presented("router.lan", 0, 2),
        true,
    );
    assert_eq!(answer.0, Answer::Cancel);
    let answer = answered(
        Some(&pin),
        "https://router.lan/",
        &presented("router.lan", 0, 1),
        true,
    );
    assert_eq!(answer.0, Answer::Accept);
}

#[test]
fn d_4_a_pin_holds_whatever_the_challenge_spells_its_host() {
    let spellings = [
        (
            "https://bücher.lan/",
            ["xn--bcher-kva.lan", "BÜCHER.lan", "bücher.lan."],
        ),
        (
            "https://[fd00::1]:8443/",
            ["fd00::1", "[fd00:0::1]", "FD00:0:0:0:0:0:0:1"],
        ),
        (
            "https://127.0.0.1:8443/",
            ["127.0.0.1", "0x7f.0.0.1", "2130706433"],
        ),
    ];
    for (url, hosts) in spellings {
        let pin = pin(url);
        let port = isize::try_from(pin.origin.port).unwrap();
        for host in hosts {
            let answer = answered(Some(&pin), url, &presented(host, port, 2), true);
            assert_eq!(answer.0, Answer::Cancel, "{url} {host}");
            let answer = answered(Some(&pin), url, &presented(host, port, 1), true);
            assert_eq!(answer.0, Answer::Accept, "{url} {host}");
        }
    }
}
