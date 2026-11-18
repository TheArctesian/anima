//! The operational keys between a daemon and its hub (K-E5 Section 2.1, R-147): the versions a
//! daemon holds and the hub's current ones, which ride on a push, and the fetch of one key.
//!
//! A [`KeyValue`] is the one type that holds a value. It implements neither `Debug`, `Display`,
//! `Clone` nor `PartialEq`, and wipes on drop (LOG-007). It leaves only through
//! [`KeyValue::expose`], and through the `value` member of [`FetchedKey`] as base64.

use std::collections::BTreeSet;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use uuid::Uuid;
use zeroize::Zeroizing;

/// The most bytes a value has (K-E5 Section 2).
pub const MAX_VALUE_BYTES: usize = 16 * 1024;

/// The most keys one daemon reports or is told of in one body.
pub const MAX_KEYS: usize = 256;

/// The longest key name, in bytes: `^[a-z][a-z0-9_.-]{0,79}$` (K-E5 Section 2).
const MAX_NAME_BYTES: usize = 80;

/// Whether `text` is the name of an operational key: a lower-case letter, then at most 79
/// lower-case letters, digits, `_`, `.` or `-` (K-E5 Section 2).
#[must_use]
pub fn is_key_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() <= MAX_NAME_BYTES
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes
            .iter()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
}

/// One key and one of its version numbers. It names a value; it never holds one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyVersion {
    pub name: String,
    pub version: u32,
}

/// The `keys` member of a `Batch`: the versions in the daemon's cache.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchKeys {
    pub held: Vec<KeyVersion>,
}

/// The `keys` member of the hub's `Accepted` answer: each key this device may get now, with its
/// active version. A key that is not in the list is not for this device now.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysCurrent {
    pub current: Vec<KeyVersion>,
}

/// The hub's `200` answer to a push (K8 Section 8.2, `Accepted`). Each member is optional, and a
/// member the daemon does not know is ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushAccepted {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<KeysCurrent>,
}

/// `POST /daemon/v1/keys/fetch`: the device of the bearer token and the name of one key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyFetch {
    pub device: Uuid,
    pub name: String,
}

/// The answer to a fetch: the active version of the key and its value.
#[derive(Serialize, Deserialize)]
pub struct FetchedKey {
    pub name: String,
    pub version: u32,
    #[serde(serialize_with = "write_value", deserialize_with = "read_value")]
    pub value: KeyValue,
}

/// The bytes of one version of an operational key, wiped on drop.
pub struct KeyValue(Zeroizing<Vec<u8>>);

impl KeyValue {
    /// The value of `bytes`; `None` unless it has 1 to [`MAX_VALUE_BYTES`] bytes.
    #[must_use]
    pub fn new(bytes: Zeroizing<Vec<u8>>) -> Option<Self> {
        (1..=MAX_VALUE_BYTES)
            .contains(&bytes.len())
            .then_some(Self(bytes))
    }

    /// The bytes, for the one place that uses them: the cache, or the standard output of
    /// `anima-daemon key read`. Never log them (LOG-007).
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    /// The value as standard base64, in a buffer wiped on drop.
    #[must_use]
    pub fn to_base64(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(
            base64::encoded_len(self.0.len(), true).unwrap_or_default(),
        ));
        STANDARD.encode_string(&*self.0, &mut text);
        text
    }

    /// The value that the standard base64 `text` spells; `None` for text that is not base64 or
    /// a value of a length a key cannot have. Nothing of the text is kept on a failure.
    #[must_use]
    pub fn from_base64(text: &str) -> Option<Self> {
        if text.len() > base64::encoded_len(MAX_VALUE_BYTES, true)? {
            return None;
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(text.len() / 4 * 3 + 3));
        STANDARD.decode_vec(text, &mut bytes).ok()?;
        Self::new(bytes)
    }
}

fn write_value<S: Serializer>(value: &KeyValue, out: S) -> Result<S::Ok, S::Error> {
    out.serialize_str(&value.to_base64())
}

/// Reads a value from a body. The error never holds the text.
fn read_value<'de, D: Deserializer<'de>>(input: D) -> Result<KeyValue, D::Error> {
    let text = Zeroizing::new(String::deserialize(input)?);
    KeyValue::from_base64(&text).ok_or_else(|| D::Error::custom("not a key value"))
}

/// Whether a list of versions is one a daemon or a hub may send: at most [`MAX_KEYS`] keys,
/// each a key name, no name twice, each version at least 1.
#[must_use]
pub fn versions_fit(list: &[KeyVersion]) -> bool {
    let names: BTreeSet<&str> = list.iter().map(|one| one.name.as_str()).collect();
    list.len() <= MAX_KEYS
        && names.len() == list.len()
        && list
            .iter()
            .all(|one| is_key_name(&one.name) && one.version >= 1)
}
