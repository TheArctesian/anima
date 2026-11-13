//! The arguments and cursors of `Query.deliveries` (NOT-004, API-006): `first` is 1 to 200, and
//! a cursor is read strictly, never trusted, never a panic.

use async_graphql::ID;
use chrono::DateTime;
use proptest::prelude::*;
use uuid::Uuid;

use crate::fleet::notifications::{
    DeliveryCursor,
    graphql::deliveries::{DEFAULT_FIRST, decode_cursor, deliveries_args, encode_cursor},
};

const HEX: &str = "0190a0000000700080030000000000aa";

fn fields(channel: Option<&ID>, first: Option<i32>, after: Option<&str>) -> Vec<&'static str> {
    deliveries_args(channel, first, after)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|error| error.field)
        .collect()
}

#[test]
fn not_004_first_defaults_to_50_and_takes_1_to_200() {
    let args = deliveries_args(None, Some(DEFAULT_FIRST), None).unwrap();
    assert_eq!((args.size, args.channel, args.after), (50, None, None));
    assert_eq!(deliveries_args(None, Some(1), None).unwrap().size, 1);
    assert_eq!(deliveries_args(None, Some(200), None).unwrap().size, 200);
    for first in [0, -1, 201, i32::MIN, i32::MAX] {
        assert_eq!(fields(None, Some(first), None), ["first"], "{first}");
    }
}

#[test]
fn not_004_a_channel_id_is_read_and_a_bad_one_named() {
    let id = Uuid::now_v7();
    let args = deliveries_args(Some(&ID(id.to_string())), None, None).unwrap();
    assert_eq!(args.channel.map(Uuid::from), Some(id));
    assert_eq!(
        fields(Some(&ID("not-an-id".into())), None, None),
        ["channelId"]
    );
}

#[test]
fn api_006_every_bad_argument_is_named_at_once() {
    assert_eq!(
        fields(Some(&ID("x".into())), Some(0), Some("x")),
        ["channelId", "first", "after"]
    );
}

#[test]
fn api_006_a_cursor_reads_back_as_its_position() {
    let cursor = DeliveryCursor {
        at: DateTime::from_timestamp_micros(1_762_598_400_123_456).unwrap(),
        id: Uuid::try_parse(HEX).unwrap().into(),
    };

    let text = encode_cursor(cursor);

    assert_eq!(text, format!("d1.1762598400123456.{HEX}"));
    assert_eq!(decode_cursor(&text), Some(cursor));
}

#[test]
fn api_006_a_cursor_of_another_shape_is_refused() {
    for text in [
        String::new(),
        "d1.".into(),
        "d1.12".into(),
        format!("d2.12.{HEX}"),
        format!("e1.12.{HEX}"),
        format!("d1..{HEX}"),
        format!("d1.-.{HEX}"),
        format!("d1.+12.{HEX}"),
        format!("d1. 12.{HEX}"),
        format!("d1.12.{HEX}0"),
        format!("d1.12.{}", &HEX[1..]),
        "d1.12.0190a000-0000-7000-8003-0000000000aa".into(),
        format!("d1.99999999999999999999.{HEX}"),
        format!("d1.9223372036854775807.{HEX}"),
        format!("d1.12.{HEX}.extra"),
        format!("d1.012.{HEX}"),
        format!("d1.-0.{HEX}"),
        format!("d1.12.{}", HEX.to_ascii_uppercase()),
        format!("d1.١٢.{HEX}"),
    ] {
        assert_eq!(decode_cursor(&text), None, "{text}");
        assert_eq!(fields(None, None, Some(&text)), ["after"], "{text}");
    }
}

proptest! {
    #[test]
    fn api_006_every_written_cursor_reads_back(
        micros in -8_000_000_000_000_000_i64..8_000_000_000_000_000_i64,
        id in any::<u128>(),
    ) {
        let cursor = DeliveryCursor {
            at: DateTime::from_timestamp_micros(micros).unwrap(),
            id: Uuid::from_u128(id).into(),
        };
        prop_assert_eq!(decode_cursor(&encode_cursor(cursor)), Some(cursor));
    }

    #[test]
    fn api_006_any_text_is_read_without_a_panic(text in any::<String>()) {
        let _ = decode_cursor(&text);
    }

    #[test]
    fn api_006_any_text_after_the_prefix_is_read_without_a_panic(
        text in "d1\\.[-+0-9a-fA-F.]{0,64}",
    ) {
        if let Some(cursor) = decode_cursor(&text) {
            prop_assert_eq!(encode_cursor(cursor), text);
        }
    }
}
