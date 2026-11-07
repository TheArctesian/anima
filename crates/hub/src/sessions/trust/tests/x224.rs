//! The X.224 exchange that starts an RDP connection (SEC-007, MS-RDPBCGR 2.2.1.1, 2.2.1.2):
//! the request's bytes, what each confirm means, and that no reply panics the parser.

use proptest::prelude::*;

use super::super::x224::{
    MAX_CONFIRM, Malformed, Negotiated, PROTOCOL_HYBRID, PROTOCOL_HYBRID_EX, PROTOCOL_RDP,
    PROTOCOL_SSL, TLS_PROTOCOLS, TPKT_HEADER, confirm_length, connection_request, negotiated,
};

/// A confirm's bytes after the TPKT header: no negotiation part when `negotiation` is empty.
fn confirm(negotiation: &[u8]) -> Vec<u8> {
    let mut body = vec![
        u8::try_from(6 + negotiation.len()).unwrap(),
        0xd0,
        0,
        0,
        0x12,
        0x34,
        0,
    ];
    body.extend_from_slice(negotiation);
    body
}

fn negotiation(kind: u8, value: u32) -> Vec<u8> {
    let mut bytes = vec![kind, 0, 8, 0];
    bytes.extend_from_slice(&value.to_le_bytes());
    bytes
}

#[test]
fn sec_007_the_request_asks_for_tls_nla_and_nla_ext_and_names_no_user() {
    let expected = [
        3, 0, 0, 19, // TPKT version 3, 19 bytes
        14, 0xe0, 0, 0, 0, 0, 0, // X.224 Connection Request, no cookie
        1, 0, 8, 0, 0x0b, 0, 0, 0, // RDP Negotiation Request: SSL | HYBRID | HYBRID_EX
    ];
    assert_eq!(connection_request(TLS_PROTOCOLS), expected);
}

#[test]
fn sec_007_a_tpkt_header_gives_the_confirm_length_within_the_limit() {
    assert_eq!(confirm_length([3, 0, 0, 19]), Ok(15));
    assert_eq!(confirm_length([3, 0, 0, 11]), Ok(7));
    let limit = u8::try_from(MAX_CONFIRM).unwrap();
    assert_eq!(
        confirm_length([3, 0, 0, limit]),
        Ok(MAX_CONFIRM - TPKT_HEADER)
    );
    for refused in [
        [3, 0, 0, 10],
        [3, 0, 0, limit + 1],
        [3, 0, 0xff, 0xff],
        [2, 0, 0, 19],
    ] {
        assert_eq!(confirm_length(refused), Err(Malformed), "{refused:?}");
    }
}

#[test]
fn sec_007_a_confirm_names_the_protocol_the_server_chose() {
    let tls = confirm(&negotiation(2, PROTOCOL_HYBRID));
    assert_eq!(
        negotiated(&tls, TLS_PROTOCOLS),
        Ok(Negotiated::Tls(PROTOCOL_HYBRID))
    );
    let ssl = confirm(&negotiation(2, PROTOCOL_SSL));
    assert_eq!(
        negotiated(&ssl, TLS_PROTOCOLS),
        Ok(Negotiated::Tls(PROTOCOL_SSL))
    );
}

#[test]
fn sec_007_standard_rdp_security_and_an_older_server_have_no_certificate() {
    let chose_rdp = confirm(&negotiation(2, PROTOCOL_RDP));
    assert_eq!(
        negotiated(&chose_rdp, TLS_PROTOCOLS),
        Ok(Negotiated::Standard)
    );
    assert_eq!(
        negotiated(&confirm(&[]), TLS_PROTOCOLS),
        Ok(Negotiated::Standard)
    );
    assert_eq!(
        negotiated(&confirm(&[]), PROTOCOL_RDP),
        Ok(Negotiated::Standard)
    );
}

#[test]
fn sec_007_a_negotiation_failure_keeps_its_code() {
    // SSL_NOT_ALLOWED_BY_SERVER
    let refused = confirm(&negotiation(3, 2));
    assert_eq!(
        negotiated(&refused, TLS_PROTOCOLS),
        Ok(Negotiated::Failure(2))
    );
}

#[test]
fn sec_007_a_choice_the_hub_did_not_ask_for_is_malformed() {
    // TLS chosen when the hub asked for standard security only, and RDSTLS (0x4), never asked.
    let tls = confirm(&negotiation(2, PROTOCOL_SSL));
    assert_eq!(negotiated(&tls, PROTOCOL_RDP), Err(Malformed));
    let rdstls = confirm(&negotiation(2, 0x4));
    assert_eq!(negotiated(&rdstls, TLS_PROTOCOLS), Err(Malformed));
}

#[test]
fn sec_007_replies_that_are_not_a_confirm_are_malformed() {
    let mut request_code = confirm(&negotiation(2, PROTOCOL_SSL));
    request_code[1] = 0xe0;
    let mut long_negotiation = negotiation(2, PROTOCOL_SSL);
    long_negotiation[2] = 9;
    let mut indicator_past_end = confirm(&[]);
    indicator_past_end[0] = 20;
    let cases = [
        Vec::new(),
        vec![0],
        request_code,
        confirm(&long_negotiation),
        confirm(&negotiation(7, PROTOCOL_SSL)),
        confirm(&[2, 0, 8]),
        indicator_past_end,
    ];
    for case in cases {
        assert_eq!(negotiated(&case, TLS_PROTOCOLS), Err(Malformed), "{case:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4_000))]

    #[test]
    fn sec_007_no_reply_panics_the_parser(
        header in any::<[u8; TPKT_HEADER]>(),
        body in proptest::collection::vec(any::<u8>(), 0..MAX_CONFIRM),
        requested in any::<u32>(),
    ) {
        let _ = confirm_length(header);
        let _ = negotiated(&body, requested);
    }

    #[test]
    fn sec_007_a_tls_choice_is_one_the_hub_asked_for(
        body in proptest::collection::vec(any::<u8>(), 0..MAX_CONFIRM),
        requested in prop_oneof![Just(PROTOCOL_RDP), Just(TLS_PROTOCOLS)],
    ) {
        if let Ok(Negotiated::Tls(chosen)) = negotiated(&body, requested) {
            prop_assert_eq!(chosen & !requested, 0);
            prop_assert_ne!(chosen & TLS_PROTOCOLS, 0);
        }
    }

    #[test]
    fn sec_007_any_well_formed_response_reads_back(chosen in prop_oneof![
        Just(PROTOCOL_SSL), Just(PROTOCOL_HYBRID), Just(PROTOCOL_HYBRID_EX)
    ]) {
        let body = confirm(&negotiation(2, chosen));
        prop_assert_eq!(negotiated(&body, TLS_PROTOCOLS), Ok(Negotiated::Tls(chosen)));
    }
}
