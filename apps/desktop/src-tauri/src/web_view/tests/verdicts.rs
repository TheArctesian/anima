//! D-4: every verdict on a server-trust challenge.

use crate::web_view::{Challenge, Fingerprint, Pin, PinnedOrigin, Verdict, verdict};

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
