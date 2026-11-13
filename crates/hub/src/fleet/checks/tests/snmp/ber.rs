//! BER encodings against X.690 by hand, round trips, and decoders that never panic on any
//! input (D-5).

use proptest::prelude::*;

use crate::fleet::checks::{
    config::oid_arcs,
    probe::snmp::{
        ber::{self, Malformed, Reader},
        community,
        message::{self, Engine, Ids},
        pdu::{self, Pdu, Value},
        usm::Keys,
    },
};

fn read_integer(bytes: &[u8]) -> Result<i64, Malformed> {
    Reader::new(bytes).integer()
}

#[test]
fn d_5_integers_use_the_shortest_twos_complement_form() {
    let cases: [(i64, &[u8]); 8] = [
        (0, &[2, 1, 0]),
        (127, &[2, 1, 0x7f]),
        (128, &[2, 2, 0, 0x80]),
        (256, &[2, 2, 1, 0]),
        (-1, &[2, 1, 0xff]),
        (-128, &[2, 1, 0x80]),
        (-129, &[2, 2, 0xff, 0x7f]),
        (i64::from(i32::MAX), &[2, 4, 0x7f, 0xff, 0xff, 0xff]),
    ];
    for (value, encoded) in cases {
        assert_eq!(
            ber::integer(ber::INTEGER, value).as_slice(),
            encoded,
            "{value}"
        );
        assert_eq!(read_integer(encoded), Ok(value), "{value}");
    }
}

#[test]
fn d_5_lengths_switch_to_the_long_form_at_128() {
    let short = ber::octets(&[7; 127]);
    assert_eq!(&short[..2], &[4, 0x7f]);
    let long = ber::octets(&[7; 200]);
    assert_eq!(&long[..3], &[4, 0x81, 200]);
    let longer = ber::octets(&[7; 300]);
    assert_eq!(&longer[..4], &[4, 0x82, 1, 0x2c]);
    let read = Reader::new(&longer).octets().unwrap();
    assert_eq!(read.len(), 300);
}

#[test]
fn d_5_an_oid_encodes_its_first_two_arcs_in_one_byte() {
    // sysUpTime.0, as every SNMP capture shows it.
    let arcs = oid_arcs("1.3.6.1.2.1.1.3.0").unwrap();
    let encoded = ber::oid(&arcs).unwrap();
    assert_eq!(encoded.as_slice(), [6, 8, 0x2b, 6, 1, 2, 1, 1, 3, 0]);
    // A large arc takes several base-128 groups.
    let encoded = ber::oid(&[1, 3, 6, 1, 4, 1, 2021, 4_294_967_295]).unwrap();
    assert_eq!(
        encoded.as_slice(),
        [
            6, 12, 0x2b, 6, 1, 4, 1, 0x8f, 0x65, 0x8f, 0xff, 0xff, 0xff, 0x7f
        ]
    );
    let read = Reader::new(&encoded).next().unwrap().oid().unwrap();
    assert_eq!(read, [1, 3, 6, 1, 4, 1, 2021, 4_294_967_295]);
    assert_eq!(
        ber::oid(&[2, 999, 3]).map(|b| b.to_vec()),
        Some(vec![6, 3, 0x88, 0x37, 3])
    );
    assert!(ber::oid(&[1, 40]).is_none());
    assert!(ber::oid(&[3, 1]).is_none());
    assert!(ber::oid(&[1]).is_none());
}

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
        .collect()
}

/// Requests captured from net-snmp 5.6.2.1 `snmpget` on a local UDP listener; the ids are the
/// ones it drew.
const SNMPGET_V2C: &str = "302902010104067075626c6963a01c0204505b88dc020100020100300e300c\
                           06082b060102010103000500";
const SNMPGET_DISCOVERY: &str = "303e0201033011020408a23569020300ffe304010402010304\
                                 10300e0400020100020100040004000400301404000400a00e02\
                                 040256faf00201000201003000";

#[test]
fn d_5_a_v2c_get_request_is_the_bytes_snmpget_sends() {
    let arcs = oid_arcs("1.3.6.1.2.1.1.3.0").unwrap();
    let request = community::request(b"public", &arcs, 0x505b_88dc).unwrap();
    assert_eq!(request.as_slice(), bytes(SNMPGET_V2C));
}

#[test]
fn d_5_engine_discovery_is_the_request_snmpget_sends() {
    let ids = Ids {
        message: 0x08a2_3569,
        request: 0x0256_faf0,
    };
    assert_eq!(message::discovery(ids).as_slice(), bytes(SNMPGET_DISCOVERY));
}

#[test]
fn d_5_reader_refuses_lengths_it_cannot_trust() {
    let refused: [&[u8]; 7] = [
        &[],
        &[4],
        &[4, 5, 1, 2],
        &[4, 0x80],
        &[4, 0x85, 0, 0, 0, 0, 1, 0],
        &[4, 0x84, 0xff, 0xff, 0xff, 0xff],
        &[0x1f, 1, 0],
    ];
    for bytes in refused {
        assert!(Reader::new(bytes).next().is_err(), "{bytes:?}");
    }
    assert!(read_integer(&[2, 0]).is_err());
    assert!(read_integer(&[2, 9, 1, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(
        Reader::new(&[6, 2, 0x2b, 0x86])
            .next()
            .unwrap()
            .oid()
            .is_err()
    );
    assert!(
        Reader::new(&[6, 3, 0x2b, 0x80, 1])
            .next()
            .unwrap()
            .oid()
            .is_err()
    );
    assert!(Reader::new(&[6, 0]).next().unwrap().oid().is_err());
}

#[test]
fn d_5_value_text_forms() {
    let cases = [
        (Value::Integer(-42), Some("-42")),
        (Value::Unsigned(4_294_967_295), Some("4294967295")),
        (Value::Octets(b"nas-01".to_vec()), Some("nas-01")),
        (Value::Octets(vec![0x00, 0x1a, 0xff]), Some("00 1A FF")),
        (
            Value::Oid(vec![1, 3, 6, 1, 4, 1, 8072]),
            Some("1.3.6.1.4.1.8072"),
        ),
        (Value::IpAddress([192, 168, 1, 10]), Some("192.168.1.10")),
        (Value::Opaque(vec![0x9f, 0x78]), Some("9F 78")),
        (Value::NoSuchObject, None),
        (Value::NoSuchInstance, None),
        (Value::EndOfMibView, None),
        (Value::Null, None),
    ];
    for (value, text) in cases {
        assert_eq!(value.text().as_deref(), text, "{value:?}");
    }
}

fn arcs() -> impl Strategy<Value = Vec<u32>> {
    (
        0_u32..3,
        0_u32..40,
        prop::collection::vec(any::<u32>(), 0..20),
    )
        .prop_map(|(first, second, rest)| {
            let mut arcs = vec![first, second];
            arcs.extend(rest);
            arcs
        })
}

fn engine() -> Engine {
    Engine {
        id: vec![0x80, 0, 0x1f, 0x88, 4],
        boots: 1,
        time: 2,
    }
}

proptest! {
    #[test]
    fn d_5_integers_round_trip(value in any::<i64>()) {
        prop_assert_eq!(read_integer(&ber::integer(ber::INTEGER, value)), Ok(value));
    }

    #[test]
    fn d_5_oids_round_trip(arcs in arcs()) {
        let encoded = ber::oid(&arcs).unwrap();
        prop_assert_eq!(Reader::new(&encoded).next().unwrap().oid().unwrap(), arcs.clone());
        let dotted = pdu::dotted(&arcs);
        prop_assert_eq!(oid_arcs(&dotted), Some(arcs));
    }

    #[test]
    fn d_5_octet_strings_round_trip(bytes in prop::collection::vec(any::<u8>(), 0..700)) {
        let encoded = ber::octets(&bytes);
        prop_assert_eq!(Reader::new(&encoded).octets().unwrap(), bytes.as_slice());
    }

    #[test]
    fn d_5_no_decoder_panics_on_any_datagram(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        let keys = Keys::localised(b"password1", b"password2", &engine().id).unwrap();
        let ids = Ids { message: 1, request: 2 };
        let _ = community::response(&bytes, 1);
        let _ = message::engine(&bytes, ids);
        let _ = message::response(&bytes, (&keys, &engine()), b"user", ids);
        let mut reader = Reader::new(&bytes);
        while let Ok(tlv) = reader.next() {
            let _ = (tlv.integer(), tlv.unsigned(), tlv.oid(), Pdu::read(&tlv));
            let _ = Pdu::last(&mut tlv.reader());
        }
    }

    #[test]
    fn d_5_no_decoder_panics_on_a_mutated_reply(
        at in 0_usize..64, byte in any::<u8>(), cut in 0_usize..64,
    ) {
        // A well-formed Response with one byte changed and its end cut, so the fuzzing
        // reaches past the outer layers.
        let mut reply = response_bytes();
        let at = at % reply.len();
        reply[at] = byte;
        reply.truncate(reply.len() - cut % reply.len());
        let _ = community::response(&reply, 7);
    }
}

/// A v2c Response with request-id 7 and one OCTET STRING binding.
fn response_bytes() -> Vec<u8> {
    let name = ber::oid(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap();
    let value = ber::octets(b"nas-01");
    let binding = ber::tlv(ber::SEQUENCE, &[&name, &value]);
    let list = ber::tlv(ber::SEQUENCE, &[&binding]);
    let numbers = [7, 0, 0].map(|n| ber::integer(ber::INTEGER, n));
    let pdu = ber::tlv(
        pdu::RESPONSE,
        &[&numbers[0], &numbers[1], &numbers[2], &list],
    );
    let version = ber::integer(ber::INTEGER, 1);
    ber::tlv(ber::SEQUENCE, &[&version, &ber::octets(b"public"), &pdu]).to_vec()
}

#[test]
fn d_5_a_response_reads_back_to_its_binding() {
    let pdu = community::response(&response_bytes(), 7).unwrap().unwrap();
    assert_eq!(
        pdu.bindings,
        [(
            vec![1, 3, 6, 1, 2, 1, 1, 5, 0],
            Value::Octets(b"nas-01".to_vec())
        )]
    );
    assert!(community::response(&response_bytes(), 8).is_none());
    let mut trailing = response_bytes();
    trailing.push(0);
    assert!(matches!(community::response(&trailing, 7), Some(Err(_))));
}
