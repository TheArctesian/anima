//! Every change to a recording file reads as damage, after the parts before it verified
//! (K4 §11, §12): the counter and the header as associated data bind each part.

use std::io;

use uuid::Uuid;

use super::{End, Sealed, damaged, read, read_from, recording, seal};
use crate::{
    ids::RecordingId,
    sessions::recording::format::{
        Damage, Format, HEADER_LEN, Header, MAX_PART, ReadError, Reader, RecordingKey,
    },
};

fn three() -> Sealed {
    seal(&[b"alpha", b"bravo"], b"charlie")
}

fn file(header: &[u8], parts: &[&Vec<u8>]) -> Vec<u8> {
    let mut bytes = header.to_vec();
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes
}

#[test]
fn ra_040_a_changed_ciphertext_byte_fails_its_part_after_the_parts_before_it() {
    let sealed = three();
    let mut bytes = sealed.bytes();
    let second = HEADER_LEN + sealed.parts[0].len() + 4;
    bytes[second] ^= 0x01;

    let (parts, end) = read(&bytes);

    assert_eq!(parts, [b"alpha".to_vec()]);
    assert_eq!(end, damaged(Damage::Tag));
}

#[test]
fn ra_040_a_changed_tag_fails() {
    let sealed = three();
    let mut bytes = sealed.bytes();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x80;

    assert_eq!(read(&bytes).1, damaged(Damage::Tag));
}

#[test]
fn ra_040_a_changed_length_is_damage() {
    let sealed = three();
    let shorter = {
        let mut bytes = sealed.bytes();
        bytes[HEADER_LEN + 3] -= 1;
        bytes
    };
    let below_a_tag = {
        let mut bytes = sealed.bytes();
        bytes[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(&15_u32.to_be_bytes());
        bytes
    };

    assert_eq!(read(&shorter).1, damaged(Damage::Tag));
    assert_eq!(read(&below_a_tag).1, damaged(Damage::Length));
}

#[test]
fn ra_040_an_oversized_length_is_refused_before_its_bytes_are_read() {
    let sealed = three();
    let over = u32::try_from(MAX_PART + 16 + 1).unwrap();
    for length in [over, u32::MAX] {
        let mut bytes = sealed.header.to_vec();
        bytes.extend_from_slice(&length.to_be_bytes());

        // No ciphertext follows: a reader that read or allocated first would see truncation.
        assert_eq!(read(&bytes).1, damaged(Damage::Length));
    }
}

#[test]
fn ra_040_a_changed_header_fails_every_part_through_the_associated_data() {
    let sealed = three();
    let mut prefix = sealed.bytes();
    prefix[HEADER_LEN - 1] ^= 0x01;
    let mut format = sealed.bytes();
    format[7] = 2;

    assert_eq!(read(&prefix), (Vec::new(), damaged(Damage::Tag)));
    assert_eq!(read(&format), (Vec::new(), damaged(Damage::Tag)));
}

#[test]
fn ra_040_parts_swapped_dropped_or_repeated_fail_at_the_first_out_of_place_part() {
    let s = three();
    let (a, b, c) = (&s.parts[0], &s.parts[1], &s.parts[2]);
    let swapped = file(&s.header, &[b, a, c]);
    let dropped = file(&s.header, &[a, c]);
    let repeated = file(&s.header, &[a, a, b, c]);

    assert_eq!(read(&swapped), (Vec::new(), damaged(Damage::Tag)));
    assert_eq!(
        read(&dropped),
        (vec![b"alpha".to_vec()], damaged(Damage::Tag))
    );
    assert_eq!(
        read(&repeated),
        (vec![b"alpha".to_vec()], damaged(Damage::Tag))
    );
}

#[test]
fn ra_040_a_part_from_another_recording_fails() {
    let ours = three();
    let other_header = Header::new(Format::Guacamole, recording(), [1; 19]);
    let mut sealer = super::Sealer::new(super::key(), &other_header);
    let theirs = sealer.seal(b"alpha").unwrap();

    assert_eq!(
        read(&file(&ours.header, &[&theirs])).1,
        damaged(Damage::Tag)
    );
}

#[test]
fn ra_040_a_file_cut_inside_a_part_is_truncated() {
    let bytes = three().bytes();
    let inside_length = HEADER_LEN + 2;
    let inside_ciphertext = bytes.len() - 5;

    assert_eq!(read(&bytes[..inside_length]).1, damaged(Damage::Truncated));
    assert_eq!(
        read(&bytes[..inside_ciphertext]),
        (
            vec![b"alpha".to_vec(), b"bravo".to_vec()],
            damaged(Damage::Truncated)
        )
    );
}

#[test]
fn ra_040_bytes_after_the_last_part_are_damage_after_every_part() {
    let sealed = three();
    let mut bytes = sealed.bytes();
    bytes.push(0);
    let mut again = sealed.bytes();
    again.extend_from_slice(&sealed.parts[2]);

    let (parts, end) = read(&bytes);

    assert_eq!(parts.len(), 3);
    assert_eq!(end, damaged(Damage::AfterLast));
    assert_eq!(read(&again).1, damaged(Damage::AfterLast));
}

#[test]
fn ra_040_a_last_part_needs_its_flag_and_a_flagged_part_ends_the_file() {
    let s = three();
    let without_last = file(&s.header, &[&s.parts[0], &s.parts[1]]);
    let early_last = seal(&[], b"alpha");
    let flagged_then_more = file(&s.header, &[&early_last.parts[0], &s.parts[1]]);

    assert_eq!(read(&without_last).1, End::Incomplete);
    assert_eq!(
        read(&flagged_then_more),
        (vec![b"alpha".to_vec()], damaged(Damage::AfterLast))
    );
}

#[test]
fn ra_040_the_wrong_key_opens_nothing() {
    let bytes = three().bytes();
    let mut reader = Reader::open(
        bytes.as_slice(),
        RecordingKey::from_bytes([8; 32]),
        recording(),
    )
    .unwrap();

    assert_eq!(
        reader.next_part().err(),
        Some(ReadError::Damaged(Damage::Tag))
    );
}

#[test]
fn ra_040_a_file_for_another_recording_id_is_refused() {
    let bytes = three().bytes();
    let other = RecordingId::from(Uuid::from_u128(1));

    let refused = Reader::open(bytes.as_slice(), super::key(), other).err();

    assert_eq!(refused, Some(ReadError::Damaged(Damage::OtherRecording)));
}

#[test]
fn ra_040_a_failing_source_is_a_read_error_not_damage() {
    struct Failing;
    impl io::Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
    }

    assert_eq!(
        read_from(Failing).1,
        End::Failed(ReadError::Read(io::ErrorKind::PermissionDenied))
    );
}
