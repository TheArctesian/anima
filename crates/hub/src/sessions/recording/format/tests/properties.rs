//! Properties over any parts and any changes: the reader never trusts a length, never
//! panics, and never ends `Done` on a file the sealer did not write.

use std::io;

use proptest::prelude::*;

use super::{End, damaged, read, read_from, seal};
use crate::sessions::recording::format::{Damage, HEADER_LEN, MAX_PART};

/// Yields its bytes in chunks of the given sizes, with an interruption before each.
struct Chunked<'a> {
    bytes: &'a [u8],
    sizes: Vec<usize>,
    interrupt: bool,
}

impl io::Read for Chunked<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.interrupt = !self.interrupt;
        if self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let size = self
            .sizes
            .pop()
            .unwrap_or(1)
            .min(buffer.len())
            .min(self.bytes.len());
        let (now, rest) = self.bytes.split_at(size);
        buffer[..size].copy_from_slice(now);
        self.bytes = rest;
        Ok(size)
    }
}

fn some_parts() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<u8>)> {
    (
        prop::collection::vec(prop::collection::vec(any::<u8>(), 1..300), 0..5),
        prop::collection::vec(any::<u8>(), 0..300),
    )
}

/// The offsets in a file at which a part starts, and its end.
fn boundaries(sealed: &super::Sealed) -> Vec<usize> {
    let mut at = HEADER_LEN;
    let mut ends = vec![at];
    for part in &sealed.parts {
        at += part.len();
        ends.push(at);
    }
    ends
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn k4_11_any_parts_round_trip_through_any_chunking(
        (parts, last) in some_parts(),
        sizes in prop::collection::vec(1..100_usize, 0..50),
    ) {
        let slices: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        let bytes = seal(&slices, &last).bytes();
        let mut expected = parts.clone();
        if !last.is_empty() {
            expected.push(last);
        }

        let chunked = Chunked { bytes: &bytes, sizes, interrupt: false };

        prop_assert_eq!(read_from(chunked), (expected, End::Done));
    }

    #[test]
    fn ra_040_any_flipped_bit_is_damage(
        (parts, last) in some_parts(),
        at in any::<prop::sample::Index>(),
        bit in 0..8_u8,
    ) {
        let slices: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        let mut bytes = seal(&slices, &last).bytes();
        let at = at.index(bytes.len());
        bytes[at] ^= 1 << bit;

        let (_, end) = read(&bytes);

        prop_assert!(matches!(end, End::Failed(_)), "{end:?} after flipping byte {at}");
    }

    #[test]
    fn ra_040_a_cut_file_is_incomplete_only_between_parts(
        (parts, last) in some_parts(),
        at in any::<prop::sample::Index>(),
    ) {
        let slices: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        let sealed = seal(&slices, &last);
        let bytes = sealed.bytes();
        let cut = at.index(bytes.len());

        let (_, end) = read(&bytes[..cut]);

        let expected = if cut < HEADER_LEN {
            damaged(Damage::Header(super::super::HeaderError::Length))
        } else if boundaries(&sealed).contains(&cut) {
            End::Incomplete
        } else {
            damaged(Damage::Truncated)
        };
        prop_assert_eq!(end, expected);
    }

    #[test]
    fn ra_040_any_bytes_after_a_header_never_panic_or_end_done(
        tail in prop::collection::vec(any::<u8>(), 0..2048),
        length in 0..u32::try_from(MAX_PART + 64).unwrap(),
    ) {
        let mut bytes = seal(&[], b"").header.to_vec();
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(&tail);

        let (parts, end) = read(&bytes);

        prop_assert!(parts.is_empty());
        prop_assert!(matches!(end, End::Failed(_)), "{end:?}");
    }
}
