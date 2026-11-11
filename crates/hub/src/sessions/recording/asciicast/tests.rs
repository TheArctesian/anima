use std::time::Duration;

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use serde_json::{Value, json};

use super::Cast;

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn started() -> (Cast, Vec<u8>) {
    Cast::start(80, 24, Utc.timestamp_opt(1_700_000_000, 0).unwrap())
}

fn parse(bytes: &[u8]) -> Value {
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(
        bytes.iter().position(|b| *b == b'\n'),
        Some(bytes.len() - 1)
    );
    serde_json::from_slice(bytes).unwrap()
}

/// All output text a run of frames produced, as the recorder would write it.
fn decoded(frames: &[&[u8]]) -> String {
    let (mut cast, _) = started();
    let mut text = String::new();
    for frame in frames {
        let out = cast.output(secs(1), frame);
        if !out.is_empty() {
            text.push_str(parse(&out)[2].as_str().unwrap());
        }
    }
    let out = cast.finish(secs(2));
    if !out.is_empty() {
        text.push_str(parse(&out)[2].as_str().unwrap());
    }
    text
}

#[test]
fn k4_11_header_has_version_size_and_start_time() {
    let (_, header) = started();
    assert_eq!(
        parse(&header),
        json!({"version": 2, "width": 80, "height": 24, "timestamp": 1_700_000_000})
    );
}

#[test]
fn k4_11_output_event_is_time_o_text() {
    let (mut cast, _) = started();
    let line = cast.output(Duration::from_micros(1_500_250), b"hello");
    assert_eq!(parse(&line), json!([1.50025, "o", "hello"]));
}

#[test]
fn k4_11_resize_event_is_time_r_colsxrows() {
    let (mut cast, _) = started();
    let line = cast.resize(secs(3), 120, 40);
    assert_eq!(parse(&line), json!([3.0, "r", "120x40"]));
}

#[test]
fn k4_11_time_never_goes_backwards() {
    let (mut cast, _) = started();
    cast.output(secs(5), b"a");
    let line = cast.resize(secs(2), 10, 10);
    assert_eq!(parse(&line)[0].as_f64(), Some(5.0));
    let line = cast.output(secs(0), b"b");
    assert_eq!(parse(&line)[0].as_f64(), Some(5.0));
}

#[test]
fn k4_11_split_sequence_joins_at_every_position() {
    for text in ["é", "€", "😀", "a€b😀c"] {
        let bytes = text.as_bytes();
        for at in 0..=bytes.len() {
            let (a, b) = bytes.split_at(at);
            assert_eq!(decoded(&[a, b]), text, "{text} split at {at}");
        }
    }
}

#[test]
fn k4_11_frame_with_only_a_sequence_start_writes_no_event() {
    let (mut cast, _) = started();
    assert_eq!(cast.output(secs(1), &[0xF0, 0x9F]), Vec::<u8>::new());
    let line = cast.output(secs(2), &[0x98, 0x80]);
    assert_eq!(parse(&line), json!([2.0, "o", "😀"]));
}

#[test]
fn k4_11_bad_bytes_become_replacement_characters() {
    assert_eq!(
        decoded(&[b"a\xFFb\xC0\xAFc"]),
        "a\u{FFFD}b\u{FFFD}\u{FFFD}c"
    );
    assert_eq!(decoded(&[&[0xE2, 0x82], b"x"]), "\u{FFFD}x");
}

#[test]
fn k4_11_incomplete_sequence_at_finish_becomes_replacement_character() {
    let (mut cast, _) = started();
    assert!(cast.output(secs(1), &[b'a', 0xE2, 0x82]).len() > 1);
    let line = cast.finish(secs(2));
    assert_eq!(parse(&line), json!([2.0, "o", "\u{FFFD}"]));
}

#[test]
fn k4_11_finish_with_nothing_carried_writes_nothing() {
    let (mut cast, _) = started();
    cast.output(secs(1), b"ok");
    assert_eq!(cast.finish(secs(2)), Vec::<u8>::new());
}

#[test]
fn k4_11_control_characters_and_escape_sequences_round_trip() {
    let text = "\x1b[31mred\x1b[0m\r\n\t\"q\" \\ \u{0}\u{7}\u{8}\u{1f}\u{7f}\u{2028}\u{2029}é";
    let (mut cast, _) = started();
    let line = cast.output(secs(1), text.as_bytes());
    assert_eq!(parse(&line)[2].as_str(), Some(text));
}

proptest! {
    #[test]
    fn k4_11_decoded_output_equals_lossy_decoding_of_the_joined_frames(
        frames in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..12), 0..8)
    ) {
        let refs: Vec<&[u8]> = frames.iter().map(Vec::as_slice).collect();
        let joined: Vec<u8> = frames.concat();
        prop_assert_eq!(decoded(&refs), String::from_utf8_lossy(&joined).into_owned());
    }

    #[test]
    fn k4_11_valid_text_split_anywhere_round_trips(text in "\\PC{0,20}", at in 0usize..80) {
        let bytes = text.as_bytes();
        let (a, b) = bytes.split_at(at.min(bytes.len()));
        prop_assert_eq!(decoded(&[a, b]), text);
    }
}
