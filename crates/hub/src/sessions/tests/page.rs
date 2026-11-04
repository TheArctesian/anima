//! Session paging: page size bounds and opaque cursors (API-006).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};

use super::Inputs;
use crate::{
    ids::SessionId,
    sessions::page::{Cursor, decode_cursor, encode_cursor, page_size, split_page},
};

fn cursor(micros: i64) -> Cursor {
    Cursor {
        started_at: DateTime::<Utc>::from_timestamp_micros(micros).unwrap(),
        id: "0190a000-0000-7000-8010-000000000001"
            .parse::<SessionId>()
            .unwrap(),
    }
}

/// A well-formed cursor whose time (`i64::MAX` microseconds) is past chrono's range.
fn past_chrono_range() -> String {
    let mut bytes = [0_u8; 24];
    bytes[..8].copy_from_slice(&i64::MAX.to_be_bytes());
    format!("s1.{}", URL_SAFE_NO_PAD.encode(bytes))
}

#[test]
fn api_006_page_size_is_one_to_two_hundred() {
    assert_eq!(page_size(1).unwrap(), 1);
    assert_eq!(page_size(200).unwrap(), 200);
    for first in [0, -1, 201, i32::MIN, i32::MAX] {
        assert_eq!(page_size(first).unwrap_err().field, "first", "{first}");
    }
}

#[test]
fn api_006_a_cursor_round_trips_to_the_microsecond() {
    for micros in [1_793_782_800_123_456, 0, -1, -62_135_596_800_000_000] {
        let at = cursor(micros);
        let text = encode_cursor(at);
        assert!(text.starts_with("s1."));
        assert_eq!(decode_cursor(&text).unwrap(), at);
    }
}

#[test]
fn api_006_text_that_is_not_our_cursor_is_refused_on_after() {
    let good = encode_cursor(cursor(1_793_782_800_000_000));
    let refused = [
        String::new(),
        "s1.".to_owned(),
        good.replacen("s1.", "s2.", 1),
        good["s1.".len()..].to_owned(),
        format!("{good}A"),
        good[..good.len() - 1].to_owned(),
        format!("s1.+{}", &good[4..]),
        "n1.6162".to_owned(),
        past_chrono_range(),
        format!("s1.{}", "A".repeat(10_000)),
    ];
    for text in refused {
        assert_eq!(decode_cursor(&text).unwrap_err().field, "after", "{text}");
    }
}

#[test]
fn api_006_cursor_decoding_never_panics() {
    let mut inputs = Inputs(0x5eed_0606);
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_+/=.s1";
    for _ in 0..20_000 {
        let text = format!("s1.{}", inputs.text(alphabet, 40));
        if let Ok(decoded) = decode_cursor(&text) {
            assert_eq!(
                encode_cursor(decoded),
                text,
                "only canonical cursors decode"
            );
        }
    }
}

#[test]
fn api_006_one_extra_row_means_another_page() {
    let page = split_page(vec![1, 2, 3], 2);
    assert_eq!((page.rows, page.has_next_page), (vec![1, 2], true));
    let page = split_page(vec![1, 2], 2);
    assert_eq!((page.rows, page.has_next_page), (vec![1, 2], false));
}
