//! The canonical form of a JSON value of RFC 8785 (JCS), which the hash of a link covers
//! (LOG-022). A second implementation of RFC 8785 gives the same bytes.

use std::fmt::Write;

use serde_json::{Number, Value};

/// `value` in the canonical form of RFC 8785: no white space, the members of each object in
/// the order of their names as UTF-16 code units, strings with the escapes of RFC 8785 Section
/// 3.2.2.2, and numbers as ECMAScript writes them.
pub(super) fn canonical(value: &Value) -> String {
    let mut text = String::new();
    write_value(&mut text, value);
    text
}

fn write_value(text: &mut String, value: &Value) {
    match value {
        Value::Null => text.push_str("null"),
        Value::Bool(true) => text.push_str("true"),
        Value::Bool(false) => text.push_str("false"),
        Value::Number(number) => text.push_str(&number_text(number)),
        Value::String(string) => write_string(text, string),
        Value::Array(values) => {
            text.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                write_value(text, value);
            }
            text.push(']');
        }
        Value::Object(members) => {
            let mut names: Vec<&String> = members.keys().collect();
            names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            text.push('{');
            for (index, name) in names.into_iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                write_string(text, name);
                text.push(':');
                write_value(text, &members[name]);
            }
            text.push('}');
        }
    }
}

fn write_string(text: &mut String, string: &str) {
    text.push('"');
    for character in string.chars() {
        match character {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\u{8}' => text.push_str("\\b"),
            '\u{c}' => text.push_str("\\f"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                write!(text, "\\u{:04x}", u32::from(control)).expect("a String takes any text");
            }
            other => text.push(other),
        }
    }
    text.push('"');
}

/// An integer as its digits; any other number as ECMAScript's `Number.prototype.toString`.
fn number_text(number: &Number) -> String {
    if let Some(integer) = number.as_i64() {
        return integer.to_string();
    }
    if let Some(integer) = number.as_u64() {
        return integer.to_string();
    }
    number.as_f64().map_or_else(String::new, float_text)
}

/// ECMAScript's text of a finite double (ECMA-262, `Number::toString`, radix 10). Rust's `{:e}`
/// gives the shortest digits that read back as the same double, as ECMAScript requires.
fn float_text(float: f64) -> String {
    if float == 0.0 {
        return "0".to_owned();
    }
    let scientific = format!("{:e}", float.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always writes an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().expect("`{:e}` writes an integer exponent");
    let sign = if float < 0.0 { "-" } else { "" };
    format!("{sign}{}", place_point(&digits, exponent + 1))
}

/// The digits `d1 d2 … dk` of the value `0.d1d2…dk × 10^n`, written as ECMAScript does.
fn place_point(digits: &str, n: i32) -> String {
    let k = i32::try_from(digits.len()).expect("a double has at most 17 digits");
    let zeros = |count: i32| "0".repeat(usize::try_from(count).unwrap_or(0));
    if k <= n && n <= 21 {
        format!("{digits}{}", zeros(n - k))
    } else if 0 < n && n <= 21 {
        let (whole, fraction) = digits.split_at(usize::try_from(n).unwrap_or(0));
        format!("{whole}.{fraction}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", zeros(-n))
    } else {
        let (first, rest) = digits.split_at(1);
        let point = if rest.is_empty() { "" } else { "." };
        let exponent = n - 1;
        let sign = if exponent > 0 { "+" } else { "-" };
        format!("{first}{point}{rest}e{sign}{}", exponent.abs())
    }
}
