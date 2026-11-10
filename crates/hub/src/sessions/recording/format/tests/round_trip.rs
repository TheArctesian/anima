//! What the sealer writes, the reader gives back, part by part, then how the file ended.

use super::{End, damaged, read, recording, seal};
use crate::sessions::recording::format::{
    Damage, Format, HEADER_LEN, Header, HeaderError, MAX_PART, NONCE_PREFIX_LEN, ReadError, Reader,
    SealError, Sealer, Step,
};

#[test]
fn k4_11_several_parts_come_back_in_order_then_done() {
    let sealed = seal(&[b"first", b"second part", b"3"], b"the tail");

    let (parts, end) = read(&sealed.bytes());

    assert_eq!(
        parts,
        [&b"first"[..], b"second part", b"3", b"the tail"].map(<[u8]>::to_vec)
    );
    assert_eq!(end, End::Done);
}

#[test]
fn k4_11_an_empty_recording_is_its_header_and_an_empty_last_part() {
    let sealed = seal(&[], b"");

    let (parts, end) = read(&sealed.bytes());

    assert_eq!(sealed.bytes().len(), HEADER_LEN + 4 + 16);
    assert!(parts.is_empty());
    assert_eq!(end, End::Done);
}

#[test]
fn k4_11_a_file_that_stops_between_parts_without_a_last_part_is_incomplete() {
    let sealed = seal(&[b"one", b"two"], b"never read");
    let mut bytes = sealed.header.to_vec();
    bytes.extend_from_slice(&sealed.parts[0]);
    bytes.extend_from_slice(&sealed.parts[1]);

    let (parts, end) = read(&bytes);

    assert_eq!(parts, [b"one".to_vec(), b"two".to_vec()]);
    assert_eq!(end, End::Incomplete);
}

#[test]
fn k4_11_a_file_with_only_its_header_is_incomplete() {
    let (parts, end) = read(&seal(&[], b"").header);

    assert!(parts.is_empty());
    assert_eq!(end, End::Incomplete);
}

#[test]
fn k4_11_a_part_of_exactly_64_kib_round_trips() {
    let full = vec![0xa5; MAX_PART];
    let sealed = seal(&[&full], &full);

    let (parts, end) = read(&sealed.bytes());

    assert_eq!(sealed.parts[0].len(), 4 + MAX_PART + 16);
    assert_eq!(parts, [full.clone(), full]);
    assert_eq!(end, End::Done);
}

#[test]
fn k4_11_a_part_one_byte_over_64_kib_is_refused() {
    let over = vec![0; MAX_PART + 1];
    let mut sealer = Sealer::new(super::key(), &super::header());

    assert_eq!(sealer.seal(&over), Err(SealError::PartTooLarge));
    assert_eq!(sealer.finish(&over), Err(SealError::PartTooLarge));
}

#[test]
fn k4_11_only_the_last_part_may_be_empty() {
    let mut sealer = Sealer::new(super::key(), &super::header());

    assert_eq!(sealer.seal(b""), Err(SealError::EmptyPart));
    assert!(sealer.finish(b"").is_ok());
}

#[test]
fn k4_11_the_header_is_magic_format_byte_id_and_nonce_prefix() {
    let prefix: [u8; NONCE_PREFIX_LEN] = std::array::from_fn(|i| u8::try_from(i).unwrap());
    let header = Header::new(Format::Asciicast, recording(), prefix);

    let bytes = header.encode();

    assert_eq!(&bytes[..7], b"ANREC1\0");
    assert_eq!(bytes[7], 2);
    assert_eq!(bytes[8..24], *uuid::Uuid::from(recording()).as_bytes());
    assert_eq!(bytes[24..], prefix);
    assert_eq!(Header::parse(&bytes), Ok(header));
    let guacamole = Header::new(Format::Guacamole, recording(), prefix).encode();
    assert_eq!(guacamole[7], 1);
    assert_eq!(
        Header::parse(&guacamole).map(|h| h.format()),
        Ok(Format::Guacamole)
    );
}

#[test]
fn k4_11_a_header_with_wrong_magic_format_or_length_is_refused() {
    let good = super::header().encode();
    let mut magic = good;
    magic[5] = b'2';
    let mut unknown = good;
    unknown[7] = 0;
    let mut three = good;
    three[7] = 3;

    assert_eq!(Header::parse(&magic), Err(HeaderError::Magic));
    assert_eq!(Header::parse(&unknown), Err(HeaderError::UnknownFormat));
    assert_eq!(Header::parse(&three), Err(HeaderError::UnknownFormat));
    assert_eq!(
        Header::parse(&good[..HEADER_LEN - 1]),
        Err(HeaderError::Length)
    );
    assert_eq!(
        Header::parse(&[good.as_slice(), &[0]].concat()),
        Err(HeaderError::Length)
    );
    assert_eq!(
        read(&good[..10]).1,
        damaged(Damage::Header(HeaderError::Length))
    );
}

#[test]
fn k4_11_the_reader_reports_the_header_format_and_ends_once() {
    let sealed = seal(&[], b"x");
    let bytes = sealed.bytes();
    let mut reader = Reader::open(bytes.as_slice(), super::key(), recording()).unwrap();

    assert_eq!(reader.format(), Format::Guacamole);
    assert!(matches!(reader.next_part(), Ok(Step::Part(part)) if *part == b"x"));
    assert!(matches!(reader.next_part(), Ok(Step::Done)));
    assert_eq!(reader.next_part().err(), Some(ReadError::Ended));
}
