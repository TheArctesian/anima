//! Example-based behaviour of `encode` and `Decoder` (GW-011).

use guac_protocol::{
    DecodeError, Decoder, Instruction, MAX_ELEMENTS, MAX_INSTRUCTION_BYTES, MAX_LENGTH_DIGITS,
    encode,
};

const TUNNEL_UUID: &str = "0192f8a4-6c1e-7b3a-9d2f-5e8c1a4b7d90";

fn decode_all(bytes: &[u8]) -> Result<Vec<Instruction>, DecodeError> {
    Decoder::new().feed(bytes)
}

#[test]
fn encodes_size_instruction() {
    let size = Instruction::new("size", ["1024", "768", "96"]);
    assert_eq!(encode(&size), "4.size,4.1024,3.768,2.96;");
}

#[test]
fn encodes_internal_instruction_with_empty_opcode() {
    let internal = Instruction::new("", [TUNNEL_UUID]);
    assert_eq!(encode(&internal), format!("0.,36.{TUNNEL_UUID};"));
}

#[test]
fn encodes_ping_and_instruction_without_args() {
    let ping = Instruction::new("ping", ["1730678400000"]);
    assert_eq!(encode(&ping), "4.ping,13.1730678400000;");
    assert_eq!(
        encode(&Instruction::new("nop", Vec::<String>::new())),
        "3.nop;"
    );
}

#[test]
fn encode_counts_code_points_not_bytes() {
    assert_eq!(encode(&Instruction::new("é€", ["😀a"])), "2.é€,2.😀a;");
}

#[test]
fn gw_011_decodes_known_encodings() {
    let stream = format!("4.size,4.1024,3.768,2.96;0.,36.{TUNNEL_UUID};4.ping,13.1730678400000;");
    let decoded = decode_all(stream.as_bytes()).expect("valid stream");
    assert_eq!(
        decoded,
        vec![
            Instruction::new("size", ["1024", "768", "96"]),
            Instruction::new("", [TUNNEL_UUID]),
            Instruction::new("ping", ["1730678400000"]),
        ]
    );
}

#[test]
fn gw_011_decodes_multibyte_values_by_code_point() {
    let decoded = decode_all("2.é€,1.😀;".as_bytes()).expect("valid stream");
    assert_eq!(decoded, vec![Instruction::new("é€", ["😀"])]);
}

#[test]
fn gw_011_accepts_leading_zeros_in_length() {
    let decoded = decode_all(b"004.ping;").expect("valid stream");
    assert_eq!(
        decoded,
        vec![Instruction::new("ping", Vec::<String>::new())]
    );
}

#[test]
fn keeps_partial_instruction_until_it_completes() {
    let mut decoder = Decoder::new();
    let bytes = "4.size,2.é€;".as_bytes();
    assert_eq!(decoder.feed(&bytes[..9]), Ok(vec![]));
    assert_eq!(
        decoder.feed(&bytes[9..]),
        Ok(vec![Instruction::new("size", ["é€"])])
    );
}

#[test]
fn gw_011_rejects_non_digit_length() {
    assert_eq!(
        decode_all(b"4.ping;x.a;"),
        Err(DecodeError::InvalidLength { position: 7 })
    );
    assert_eq!(
        decode_all(b" 4.ping;"),
        Err(DecodeError::InvalidLength { position: 0 })
    );
    assert_eq!(
        decode_all(b".;"),
        Err(DecodeError::InvalidLength { position: 0 })
    );
}

#[test]
fn gw_011_rejects_too_many_length_digits() {
    let accepted = format!("{}.ab;", "0".repeat(MAX_LENGTH_DIGITS - 1) + "2");
    assert!(decode_all(accepted.as_bytes()).is_ok());
    let rejected = format!("{}.ab;", "0".repeat(MAX_LENGTH_DIGITS) + "2");
    assert_eq!(
        decode_all(rejected.as_bytes()),
        Err(DecodeError::TooManyLengthDigits {
            position: 5,
            limit: MAX_LENGTH_DIGITS
        })
    );
}

#[test]
fn gw_011_rejects_length_longer_than_value() {
    // The `;` is read as the fifth character, so the next byte must be a separator.
    assert_eq!(
        decode_all(b"5.abcd;4.ping;"),
        Err(DecodeError::MissingSeparator { position: 7 })
    );
}

#[test]
fn gw_011_rejects_length_shorter_than_value() {
    assert_eq!(
        decode_all(b"3.abcd;"),
        Err(DecodeError::MissingSeparator { position: 5 })
    );
    assert_eq!(
        decode_all("1.é€;".as_bytes()),
        Err(DecodeError::MissingSeparator { position: 4 })
    );
}

#[test]
fn gw_011_rejects_invalid_utf8() {
    assert_eq!(
        decode_all(b"1.\xff;"),
        Err(DecodeError::InvalidUtf8 { position: 3 })
    );
    // A lead byte cut short by the separator.
    assert_eq!(
        decode_all(b"1.\xe2\x82;"),
        Err(DecodeError::InvalidUtf8 { position: 4 })
    );
    assert_eq!(
        decode_all(b"0.\x80;"),
        Err(DecodeError::InvalidUtf8 { position: 3 })
    );
}

#[test]
fn gw_011_accepts_instruction_of_exactly_max_bytes() {
    // 4 digits + '.' + value + ';' == MAX_INSTRUCTION_BYTES.
    let length = MAX_INSTRUCTION_BYTES - 6;
    let stream = format!("{length}.{};", "a".repeat(length));
    assert_eq!(stream.len(), MAX_INSTRUCTION_BYTES);
    let decoded = decode_all(stream.as_bytes()).expect("instruction at the limit");
    assert_eq!(decoded[0].opcode.len(), length);
}

fn too_long_at(position: usize) -> Result<Vec<Instruction>, DecodeError> {
    Err(DecodeError::InstructionTooLong {
        position: u64::try_from(position).expect("fits"),
        limit: MAX_INSTRUCTION_BYTES,
    })
}

/// Feeds `bytes` seven bytes at a time and collects every result.
fn decode_in_chunks(bytes: &[u8]) -> Result<Vec<Instruction>, DecodeError> {
    let mut decoder = Decoder::new();
    let mut instructions = Vec::new();
    for chunk in bytes.chunks(7) {
        instructions.extend(decoder.feed(chunk)?);
    }
    Ok(instructions)
}

/// Opcode `a` and 65 arguments of 60 two-byte characters: 8063 bytes, no `;`.
fn sixty_five_wide_args() -> String {
    let arg = "é".repeat(60);
    format!("1.a{}", format!(",60.{arg}").repeat(65))
}

#[test]
fn gw_011_rejects_oversize_instruction_before_terminator() {
    // The length fits the early check; the two-byte characters overflow it.
    let head = "4100.";
    let unterminated = format!("{head}{}", "é".repeat(4100));
    assert_eq!(
        decode_in_chunks(unterminated.as_bytes()),
        too_long_at(MAX_INSTRUCTION_BYTES)
    );
}

#[test]
fn gw_011_rejects_length_that_cannot_fit_at_the_dot() {
    assert_eq!(decode_all(b"99999."), too_long_at(5));
    let over = MAX_INSTRUCTION_BYTES - 5;
    assert_eq!(decode_all(format!("{over}.").as_bytes()), too_long_at(4));
    let fits = MAX_INSTRUCTION_BYTES - 6;
    assert_eq!(decode_all(format!("{fits}.").as_bytes()), Ok(vec![]));
}

#[test]
fn gw_011_limit_counts_lengths_and_separators_of_all_elements() {
    let unterminated = format!("{},100.{}", sixty_five_wide_args(), "é".repeat(100));
    assert_eq!(
        decode_in_chunks(unterminated.as_bytes()),
        too_long_at(MAX_INSTRUCTION_BYTES)
    );
}

#[test]
fn gw_011_accepts_many_elements_of_exactly_max_bytes() {
    let stream = format!("{},123.{};", sixty_five_wide_args(), "b".repeat(123));
    assert_eq!(stream.len(), MAX_INSTRUCTION_BYTES);
    let decoded = decode_in_chunks(stream.as_bytes()).expect("instruction at the limit");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].args.len(), 66);
}

#[test]
fn gw_011_rejects_nul_in_opcode() {
    assert_eq!(
        decode_all(b"5.size\0,6.100000,6.100000,4.9999;"),
        Err(DecodeError::NulByte { position: 6 })
    );
}

#[test]
fn gw_011_rejects_nul_in_internal_opcode() {
    assert_eq!(
        decode_all(b"1.\0,4.ping,1.1;"),
        Err(DecodeError::NulByte { position: 2 })
    );
}

#[test]
fn gw_011_rejects_nul_in_argument() {
    assert_eq!(
        decode_all(b"4.size,4.10\x0024,3.768,2.96;"),
        Err(DecodeError::NulByte { position: 11 })
    );
}

#[test]
fn gw_011_limit_counts_each_instruction_separately() {
    let one = format!(
        "{}.{};",
        MAX_INSTRUCTION_BYTES - 6,
        "a".repeat(MAX_INSTRUCTION_BYTES - 6)
    );
    let decoded = decode_all(one.repeat(3).as_bytes()).expect("three instructions at the limit");
    assert_eq!(decoded.len(), 3);
}

#[test]
fn gw_011_rejects_more_than_max_elements() {
    let args = vec!["x"; MAX_ELEMENTS - 1];
    let at_limit = encode(&Instruction::new("a", args.clone()));
    assert_eq!(
        decode_all(at_limit.as_bytes()).expect("at limit")[0]
            .args
            .len(),
        MAX_ELEMENTS - 1
    );

    let mut over = args;
    over.push("x");
    let encoded = encode(&Instruction::new("a", over));
    assert!(matches!(
        decode_all(encoded.as_bytes()),
        Err(DecodeError::TooManyElements {
            limit: MAX_ELEMENTS,
            ..
        })
    ));
}

#[test]
fn gw_011_failed_decoder_stays_failed() {
    let mut decoder = Decoder::new();
    let error = decoder.feed(b"4.ping;x").expect_err("bad length");
    assert_eq!(decoder.feed(b"4.ping;"), Err(error.clone()));
    assert_eq!(decoder.feed(b""), Err(error));
}

#[test]
fn log_007_instruction_debug_hides_argument_values() {
    let connect = Instruction::new(
        "connect",
        ["VERSION_1_5_0", "host.example", "hunter2-secret"],
    );
    let printed = format!("{connect:?}");
    assert!(printed.contains("connect"));
    assert!(printed.contains('3'));
    assert!(!printed.contains("hunter2-secret"));
    assert!(!printed.contains("host.example"));
}

#[test]
fn log_007_instruction_debug_truncates_long_opcode() {
    let long = Instruction::new("x".repeat(40), ["v"]);
    let printed = format!("{long:?}");
    assert!(printed.contains(&format!("{}…", "x".repeat(32))));
    assert!(!printed.contains(&"x".repeat(33)));
    assert!(!format!("{:?}", Instruction::new("x".repeat(32), ["v"])).contains('…'));
}

#[test]
fn log_007_decoder_debug_and_errors_hide_buffered_content() {
    let mut decoder = Decoder::new();
    assert_eq!(decoder.feed(b"7.connect,14.hunter2-secret"), Ok(vec![]));
    assert!(!format!("{decoder:?}").contains("hunter2"));
    let error = decoder.feed(b"x").expect_err("missing separator");
    assert!(!error.to_string().contains("hunter2"));
    assert!(!format!("{error:?}{decoder:?}").contains("hunter2"));
}
