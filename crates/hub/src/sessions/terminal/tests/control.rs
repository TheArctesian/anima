//! The terminal's text messages (K4 Sections 7, 8.4 and 8.5): exactly the closed set is
//! accepted, nothing panics, and the hub's own messages have the contract's shape.

use async_graphql::{InputType as _, Value as GraphqlValue};
use proptest::prelude::*;
use serde_json::{Map, Value, json};

use super::super::control::{BadControl, Control, MAX_TEXT_BYTES, PING, Size, end, k1_name, parse};
use crate::sessions::EndReason;

fn resize(cols: u16, rows: u16) -> Control {
    Control::Resize(Size { cols, rows })
}

#[test]
fn k4_8_4_the_two_client_messages_are_accepted_in_any_order_and_spacing() {
    let cases = [
        (
            r#"{"type":"resize","cols":80,"rows":24}"#,
            Ok(resize(80, 24)),
        ),
        (
            r#"{"rows":24,"cols":80,"type":"resize"}"#,
            Ok(resize(80, 24)),
        ),
        (
            " {\n\"type\" : \"resize\", \"cols\":1,\"rows\":1000 }\t",
            Ok(resize(1, 1000)),
        ),
        (
            r#"{"type":"resize","cols":1000,"rows":1}"#,
            Ok(resize(1000, 1)),
        ),
        (r#"{"type":"ping"}"#, Ok(Control::Ping)),
        (r#" { "type" : "ping" } "#, Ok(Control::Ping)),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), expected, "{text}");
    }
}

#[test]
fn k4_8_4_every_other_shape_is_refused() {
    let unknown = [
        "",
        "null",
        "[]",
        r#""resize""#,
        "{}",
        r#"{"type":"end","reason":"OPERATOR"}"#,
        r#"{"type":"Resize","cols":80,"rows":24}"#,
        r#"{"type":"pong"}"#,
        r#"{"type":null}"#,
        r#"{"type":"ping","x":1}"#,
        r#"{"type":"ping","cols":null}"#,
        r#"{"type":"resize","cols":80}"#,
        r#"{"type":"resize","rows":24}"#,
        r#"{"type":"resize","cols":80,"rows":24,"x":0}"#,
        r#"{"type":"resize","cols":80,"rows":24,"type":"resize"}"#,
        r#"{"type":"resize","cols":80,"cols":81,"rows":24}"#,
        r#"{"type":"resize","cols":80.0,"rows":24}"#,
        r#"{"type":"resize","cols":8e1,"rows":24}"#,
        r#"{"type":"resize","cols":-80,"rows":24}"#,
        r#"{"type":"resize","cols":"80","rows":24}"#,
        r#"{"type":"resize","cols":null,"rows":24}"#,
        r#"{"type":"resize","cols":65536,"rows":24}"#,
        r#"{"type":"resize","cols":080,"rows":24}"#,
        r#"{"type":"resize","cols":[80],"rows":24}"#,
        r#"{"type":"ping"} {"type":"ping"}"#,
        r#"{"type":"ping"}x"#,
        "\u{feff}{\"type\":\"ping\"}",
        r#"["ping"]"#,
        r#"["resize",80,24]"#,
        r#" [ "resize", 80, 24 ] "#,
    ];
    for text in unknown {
        assert_eq!(parse(text), Err(BadControl::Unknown), "{text}");
    }
    for text in [
        r#"{"type":"resize","cols":0,"rows":24}"#,
        r#"{"type":"resize","cols":80,"rows":0}"#,
        r#"{"type":"resize","cols":1001,"rows":24}"#,
        r#"{"type":"resize","cols":80,"rows":65535}"#,
    ] {
        assert_eq!(parse(text), Err(BadControl::OutOfRange), "{text}");
    }
}

#[test]
fn k4_8_5_text_over_256_bytes_is_refused_even_when_it_is_a_valid_message() {
    let padded = |len: usize| format!("{}{}", r#"{"type":"ping"}"#, " ".repeat(len - PING.len()));
    assert_eq!(parse(&padded(MAX_TEXT_BYTES)), Ok(Control::Ping));
    assert_eq!(parse(&padded(MAX_TEXT_BYTES + 1)), Err(BadControl::TooLong));
}

#[test]
fn k4_8_4_the_hubs_messages_have_the_contracts_shape() {
    assert_eq!(
        serde_json::from_str::<Value>(PING).unwrap(),
        json!({"type": "ping"})
    );
    for reason in EndReason::ALL {
        let sent: Value = serde_json::from_str(&end(reason)).unwrap();
        assert_eq!(sent, json!({"type": "end", "reason": k1_name(reason)}));
    }
}

#[test]
fn k1_the_end_reasons_are_the_graphql_session_end_reason_names() {
    for reason in EndReason::ALL {
        assert_eq!(
            reason.to_value(),
            GraphqlValue::Enum(async_graphql::Name::new(k1_name(reason))),
        );
    }
}

/// Whether K4 Section 8.4 accepts `value` as a client message: exactly `{"type":"ping"}`, or
/// `{"type":"resize"}` with integer `cols` and `rows` each in `1..=1000`.
fn accepted(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let in_range = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_u64)
            .is_some_and(|cells| (1..=1000).contains(&cells))
    };
    match object.get("type").and_then(Value::as_str) {
        Some("ping") => object.len() == 1,
        Some("resize") => object.len() == 3 && in_range("cols") && in_range("rows"),
        _ => false,
    }
}

/// JSON values near the accepted messages: their keys and type names, numbers around the
/// bounds, floats, and nesting.
fn near_messages() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        (-2_i64..=1_002).prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        (-1.0e3_f64..1.0e4).prop_map(Value::from),
        prop_oneof![
            Just("resize"),
            Just("ping"),
            Just("end"),
            Just("RESIZE"),
            Just("")
        ]
        .prop_map(Value::from),
    ];
    leaf.prop_recursive(3, 24, 6, |inner| {
        let key = prop_oneof![
            Just("type"),
            Just("cols"),
            Just("rows"),
            Just("reason"),
            Just("x")
        ];
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::btree_map(key.prop_map(str::to_owned), inner, 0..5)
                .prop_map(|map| Value::Object(map.into_iter().collect::<Map<_, _>>())),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn k4_7_no_text_panics_the_parser(text in any::<String>()) {
        let _ = parse(&text);
    }

    #[test]
    fn k4_7_only_the_section_8_4_messages_are_accepted(value in near_messages()) {
        let text = value.to_string();
        prop_assume!(text.len() <= MAX_TEXT_BYTES);
        prop_assert_eq!(parse(&text).is_ok(), accepted(&value), "{}", text);
    }

    #[test]
    fn k4_8_4_every_size_in_range_is_accepted_with_its_values(cols in 1_u16..=1000, rows in 1_u16..=1000) {
        let text = format!(r#"{{"cols":{cols},"type":"resize","rows":{rows}}}"#);
        prop_assert_eq!(parse(&text), Ok(resize(cols, rows)));
    }

    #[test]
    fn k4_8_4_a_valid_message_with_any_extra_field_is_refused(
        name in "[a-z]{1,8}",
        message in prop_oneof![Just(r#""type":"ping""#), Just(r#""type":"resize","cols":80,"rows":24"#)],
    ) {
        prop_assume!(!["type", "cols", "rows"].contains(&name.as_str()));
        let text = format!(r#"{{{message},"{name}":1}}"#);
        prop_assert_eq!(parse(&text), Err(BadControl::Unknown));
    }
}
