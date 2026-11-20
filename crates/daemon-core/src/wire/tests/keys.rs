//! The operational keys on the wire: the `keys` member of a batch, `Accepted`, and the fetched
//! value (K-E5 Section 2.1, R-147, LOG-007).

use std::fmt::{Debug, Display};

use proptest::prelude::*;
use static_assertions::assert_not_impl_any;
use zeroize::Zeroizing;

use super::{assert_malformed, checked};
use crate::wire::{
    BatchKeys, FetchedKey, KeyValue, KeyVersion, MAX_KEYS, PushAccepted, is_key_name,
};

assert_not_impl_any!(KeyValue: Debug, Display, Clone, PartialEq);
assert_not_impl_any!(FetchedKey: Debug, Display, Clone);

fn held(name: &str, version: u32) -> KeyVersion {
    KeyVersion {
        name: name.to_owned(),
        version,
    }
}

#[test]
fn r_147_a_batch_hands_over_the_versions_its_cache_holds() {
    let list = vec![held("cloudflare.dns", 3), held("ollama.token", 1)];
    let accepted = checked(|batch| batch.keys = Some(BatchKeys { held: list.clone() })).unwrap();
    assert_eq!(accepted.keys, Some(list));
    assert_eq!(checked(|_| {}).unwrap().keys, None, "absent: no key cache");
}

#[test]
fn r_147_a_held_list_that_breaks_its_rules_is_malformed() {
    let too_many: Vec<KeyVersion> = (0..=MAX_KEYS).map(|n| held(&format!("k{n}"), 1)).collect();
    let cases = [
        ("a name twice", vec![held("a", 1), held("a", 2)]),
        ("not a key name", vec![held("A", 1)]),
        ("version 0", vec![held("a", 0)]),
        ("too many", too_many),
    ];
    for (case, list) in cases {
        assert_malformed(case, |batch| batch.keys = Some(BatchKeys { held: list }));
    }
}

#[test]
fn r_147_accepted_reads_with_or_without_keys_and_ignores_other_members() {
    let told: PushAccepted =
        serde_json::from_str(r#"{"keys":{"current":[{"name":"a","version":2}]},"later":{}}"#)
            .unwrap();
    assert_eq!(told.keys.unwrap().current, [held("a", 2)]);
    let none: PushAccepted = serde_json::from_str("{}").unwrap();
    assert_eq!(none.keys, None);
}

#[test]
fn r_147_a_fetched_value_crosses_as_base64_and_reads_back_whole() {
    let value = KeyValue::new(Zeroizing::new(vec![0, 255, b'"', b'\n'])).unwrap();
    let fetched = FetchedKey {
        name: "cloudflare.dns".to_owned(),
        version: 7,
        value,
    };
    let text = serde_json::to_string(&fetched).unwrap();
    assert_eq!(
        text,
        r#"{"name":"cloudflare.dns","version":7,"value":"AP8iCg=="}"#
    );
    let read: FetchedKey = serde_json::from_str(&text).unwrap();
    assert_eq!(read.value.expose(), [0, 255, b'"', b'\n']);
}

#[test]
fn r_147_a_value_must_have_1_to_16384_bytes() {
    assert!(KeyValue::new(Zeroizing::new(Vec::new())).is_none());
    assert!(KeyValue::new(Zeroizing::new(vec![1; 16 * 1024])).is_some());
    assert!(KeyValue::new(Zeroizing::new(vec![1; 16 * 1024 + 1])).is_none());
    let error = serde_json::from_str::<FetchedKey>(
        r#"{"name":"a","version":1,"value":"c2VjcmV0LXZhbHVl!"}"#,
    )
    .err()
    .unwrap();
    assert!(!error.to_string().contains("c2VjcmV0"), "{error}");
}

proptest! {
    /// Any text is either a value or nothing; reading never panics, and a value read back
    /// writes the same base64.
    #[test]
    fn r_147_reading_any_text_as_a_value_never_panics(text in "\\PC{0,64}") {
        if let Some(value) = KeyValue::from_base64(&text) {
            prop_assert_eq!(&*value.to_base64(), text.as_str());
        }
    }

    /// The name rule and the length: every accepted name has 1 to 80 bytes.
    #[test]
    fn r_147_a_key_name_has_its_one_form(text in "[a-zA-Z0-9_./ -]{0,90}") {
        if is_key_name(&text) {
            prop_assert!((1..=80).contains(&text.len()));
            prop_assert!(text.as_bytes()[0].is_ascii_lowercase());
        }
    }
}
