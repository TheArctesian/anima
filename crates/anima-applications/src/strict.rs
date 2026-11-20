//! The reader of JSON for a request and for the settings file. It refuses what a second
//! reader could read in a different way: a member that is there two times, a number that is
//! not a whole number of 0 or more, and text after the value.

use std::fmt;

use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};

/// One value of JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Null,
    Truth(bool),
    Whole(u64),
    Text(String),
    List(Vec<Value>),
    /// The members in the order of the text; no name is there two times.
    Record(Vec<(String, Value)>),
}

struct Reader;

impl<'de> Visitor<'de> for Reader {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a value of strict JSON")
    }

    fn visit_unit<E: Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: Error>(self, truth: bool) -> Result<Value, E> {
        Ok(Value::Truth(truth))
    }

    fn visit_u64<E: Error>(self, whole: u64) -> Result<Value, E> {
        Ok(Value::Whole(whole))
    }

    fn visit_str<E: Error>(self, text: &str) -> Result<Value, E> {
        Ok(Value::Text(text.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut list: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = list.next_element()? {
            values.push(value);
        }
        Ok(Value::List(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut record: A) -> Result<Value, A::Error> {
        let mut members: Vec<(String, Value)> = Vec::new();
        while let Some((name, value)) = record.next_entry::<String, Value>()? {
            if members.iter().any(|(known, _)| *known == name) {
                return Err(A::Error::custom("a member is there two times"));
            }
            members.push((name, value));
        }
        Ok(Value::Record(members))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Reader)
    }
}

/// The one value of `text`; `None` for text that is not strict JSON. The fault of the reader
/// is dropped: it can hold text of the input.
pub(crate) fn read(text: &str) -> Option<Value> {
    let mut reader = serde_json::Deserializer::from_str(text);
    let value = Value::deserialize(&mut reader).ok()?;
    reader.end().ok()?;
    Some(value)
}

/// The members of one record, taken one at a time, so that a member that nobody took is
/// found at the end.
pub(crate) struct Members(Vec<(String, Value)>);

impl Members {
    pub(crate) fn of(value: Value) -> Option<Self> {
        match value {
            Value::Record(members) => Some(Self(members)),
            _ => None,
        }
    }

    /// Takes the member `name`; `None` when it is not there.
    pub(crate) fn take(&mut self, name: &str) -> Option<Value> {
        let at = self.0.iter().position(|(known, _)| known == name)?;
        Some(self.0.remove(at).1)
    }

    pub(crate) fn text(&mut self, name: &str) -> Option<String> {
        match self.take(name)? {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn whole(&mut self, name: &str) -> Option<u64> {
        match self.take(name)? {
            Value::Whole(whole) => Some(whole),
            _ => None,
        }
    }

    pub(crate) fn truth(&mut self, name: &str) -> Option<bool> {
        match self.take(name)? {
            Value::Truth(truth) => Some(truth),
            _ => None,
        }
    }

    /// The texts of a member that is a list of texts.
    pub(crate) fn texts(&mut self, name: &str) -> Option<Vec<String>> {
        let Value::List(values) = self.take(name)? else {
            return None;
        };
        values
            .into_iter()
            .map(|value| match value {
                Value::Text(text) => Some(text),
                _ => None,
            })
            .collect()
    }

    /// True when each member was taken: the record had no member that the reader does not know.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
