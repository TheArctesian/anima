//! Property and fuzz-style behaviour of `Decoder` over hostile input (GW-011).

use guac_protocol::{
    DecodeError, Decoder, Instruction, MAX_ELEMENTS, MAX_INSTRUCTION_BYTES, encode,
};
use proptest::prelude::*;

use crate::arbitrary::text;

fn instruction(max_chars: usize, max_args: usize) -> impl Strategy<Value = Instruction> {
    (
        text(max_chars),
        prop::collection::vec(text(max_chars), 0..=max_args),
    )
        .prop_map(|(opcode, args)| Instruction { opcode, args })
}

fn stream(instructions: &[Instruction]) -> Vec<u8> {
    instructions
        .iter()
        .map(encode)
        .collect::<String>()
        .into_bytes()
}

/// Feeds `bytes` in chunks of the given sizes (cycled) and collects every result.
fn feed_in_chunks(bytes: &[u8], sizes: &[usize]) -> Result<Vec<Instruction>, DecodeError> {
    let mut decoder = Decoder::new();
    let mut instructions = Vec::new();
    let mut rest = bytes;
    for &size in sizes.iter().cycle() {
        if rest.is_empty() {
            break;
        }
        let (chunk, tail) = rest.split_at(size.clamp(1, rest.len()));
        instructions.extend(decoder.feed(chunk)?);
        rest = tail;
    }
    Ok(instructions)
}

/// Every instruction a decoder returns fits guacd's limits and holds no NUL.
fn assert_acceptable(decoded: &[Instruction]) {
    for i in decoded {
        assert!(encode(i).len() <= MAX_INSTRUCTION_BYTES);
        assert!(i.args.len() < MAX_ELEMENTS);
        assert!(!i.opcode.contains('\0'));
        assert!(i.args.iter().all(|arg| !arg.contains('\0')));
    }
}

/// Byte offsets of the first byte of every code point inside a value of `i`'s encoding.
fn value_byte_offsets(i: &Instruction) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut at = 0;
    for value in std::iter::once(&i.opcode).chain(&i.args) {
        at += value.chars().count().to_string().len() + 1;
        offsets.extend(value.char_indices().map(|(index, _)| at + index));
        at += value.len() + 1;
    }
    offsets
}

proptest! {
    #[test]
    fn gw_011_round_trips_any_instruction(i in instruction(64, 16)) {
        let decoded = Decoder::new().feed(encode(&i).as_bytes());
        prop_assert_eq!(decoded, Ok(vec![i]));
    }

    #[test]
    fn gw_011_decodes_several_instructions_in_one_feed(
        list in prop::collection::vec(instruction(16, 6), 0..8),
    ) {
        prop_assert_eq!(Decoder::new().feed(&stream(&list)), Ok(list));
    }

    #[test]
    fn gw_011_split_at_every_byte_boundary_gives_same_instructions(
        list in prop::collection::vec(instruction(5, 3), 1..4),
    ) {
        let bytes = stream(&list);
        for split in 0..=bytes.len() {
            let (head, tail) = bytes.split_at(split);
            let mut decoder = Decoder::new();
            let mut decoded = decoder.feed(head).expect("valid head");
            decoded.extend(decoder.feed(tail).expect("valid tail"));
            prop_assert_eq!(&decoded, &list, "split at byte {}", split);
        }
    }

    #[test]
    fn gw_011_random_chunking_gives_same_instructions(
        list in prop::collection::vec(instruction(24, 8), 1..6),
        sizes in prop::collection::vec(1usize..16, 1..16),
    ) {
        prop_assert_eq!(feed_in_chunks(&stream(&list), &sizes), Ok(list));
    }

    #[test]
    fn gw_011_byte_at_a_time_gives_same_instructions(
        list in prop::collection::vec(instruction(24, 8), 1..4),
    ) {
        prop_assert_eq!(feed_in_chunks(&stream(&list), &[1]), Ok(list));
    }

    #[test]
    fn gw_011_rejects_any_non_digit_where_length_starts(
        valid in instruction(8, 3),
        bad in any::<u8>().prop_filter("not a digit", |b| !b.is_ascii_digit()),
    ) {
        let mut bytes = encode(&valid).into_bytes();
        let position = u64::try_from(bytes.len()).expect("fits");
        bytes.push(bad);
        prop_assert_eq!(
            Decoder::new().feed(&bytes),
            Err(DecodeError::InvalidLength { position })
        );
    }

    #[test]
    fn gw_011_rejects_wrong_length_prefix(value in "[a-z]{1,40}", delta in 1usize..5) {
        let length = value.chars().count();
        let short = format!("{}.{value};", length - 1);
        let result = Decoder::new().feed(short.as_bytes());
        prop_assert!(
            matches!(result, Err(DecodeError::MissingSeparator { .. })),
            "unexpected result {:?}",
            result
        );
        // A longer length swallows the `;` and needs a separator after more input.
        let long = format!("{}.{value};{}", length + delta, "4.ping;".repeat(4));
        prop_assert!(Decoder::new().feed(long.as_bytes()).is_err());
    }

    #[test]
    fn gw_011_rejects_oversize_instruction_before_terminator(
        length in (MAX_INSTRUCTION_BYTES / 2)..(MAX_INSTRUCTION_BYTES - 6),
        sizes in prop::collection::vec(1usize..512, 1..8),
    ) {
        // Two-byte characters pass the length check at `.` and overflow later.
        let unterminated = format!("{length}.{}", "é".repeat(length));
        let result = feed_in_chunks(unterminated.as_bytes(), &sizes);
        prop_assert!(
            matches!(result, Err(DecodeError::InstructionTooLong { .. })),
            "unexpected result {:?}", result
        );
    }

    #[test]
    fn gw_011_rejects_nul_anywhere_in_a_value(
        i in instruction(8, 3),
        at in any::<prop::sample::Index>(),
    ) {
        let encoded = encode(&i);
        let value_bytes = value_byte_offsets(&i);
        prop_assume!(!value_bytes.is_empty());
        let mut bytes = encoded.into_bytes();
        let offset = value_bytes[at.index(value_bytes.len())];
        bytes[offset] = 0;
        let result = Decoder::new().feed(&bytes);
        prop_assert!(matches!(result, Err(DecodeError::NulByte { .. })), "unexpected {:?}", result);
    }

    #[test]
    fn gw_011_error_is_sticky(
        bytes in prop::collection::vec(any::<u8>(), 0..256),
        later in instruction(8, 3),
    ) {
        let mut decoder = Decoder::new();
        if let Err(error) = decoder.feed(&bytes) {
            prop_assert_eq!(decoder.feed(encode(&later).as_bytes()), Err(error));
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    #[test]
    fn gw_011_arbitrary_bytes_never_panic(
        bytes in prop::collection::vec(any::<u8>(), 0..4096),
        sizes in prop::collection::vec(1usize..64, 1..16),
    ) {
        if let Ok(decoded) = feed_in_chunks(&bytes, &sizes) {
            assert_acceptable(&decoded);
        }
    }

    #[test]
    fn gw_011_arbitrary_length_shaped_bytes_never_panic(
        bytes in prop::collection::vec(
            prop::sample::select(b"0123456789.,;a\x00\xc3\xa9\x80\xff".to_vec()),
            0..4096,
        ),
        sizes in prop::collection::vec(1usize..64, 1..16),
    ) {
        if let Ok(decoded) = feed_in_chunks(&bytes, &sizes) {
            assert_acceptable(&decoded);
        }
    }

    #[test]
    fn gw_011_mutated_valid_streams_never_panic(
        list in prop::collection::vec(instruction(16, 6), 1..6),
        mutations in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8),
        sizes in prop::collection::vec(1usize..64, 1..16),
    ) {
        let mut bytes = stream(&list);
        for (index, byte) in &mutations {
            let at = index.index(bytes.len());
            bytes[at] = *byte;
        }
        if let Ok(decoded) = feed_in_chunks(&bytes, &sizes) {
            assert_acceptable(&decoded);
        }
    }

    #[test]
    fn gw_011_truncated_or_extended_streams_never_panic(
        list in prop::collection::vec(instruction(16, 6), 1..6),
        cut in any::<prop::sample::Index>(),
        tail in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut bytes = stream(&list);
        bytes.truncate(cut.index(bytes.len() + 1));
        bytes.extend(tail);
        if let Ok(decoded) = Decoder::new().feed(&bytes) {
            assert_acceptable(&decoded);
        }
    }
}
