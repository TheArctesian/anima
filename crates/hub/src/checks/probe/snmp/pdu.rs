//! SNMP PDUs (RFC 3416): the `GetRequest` the hub sends, the Response or Report it reads, and
//! the value of a variable binding with its text form. Pure.

use std::fmt::Write as _;

use super::ber::{self, Bytes, Malformed, Reader, Tlv};
use crate::checks::probe::Failure;

pub(in crate::checks) const GET_REQUEST: u8 = 0xa0;
pub(in crate::checks) const RESPONSE: u8 = 0xa2;
pub(in crate::checks) const REPORT: u8 = 0xa8;

/// The most variable bindings read from one PDU; the hub asks for one.
const BINDINGS_MAX: usize = 8;

/// A `GetRequest` for one OID, or with no binding for engine discovery (RFC 3414 Section 4).
/// `None` when the OID cannot be encoded.
#[must_use]
pub(in crate::checks) fn get_request(request_id: i32, oid: Option<&[u32]>) -> Option<Bytes> {
    let bindings = match oid {
        Some(arcs) => {
            let name = ber::oid(arcs)?;
            let null = ber::tlv(ber::NULL, &[]);
            ber::tlv(ber::SEQUENCE, &[&ber::tlv(ber::SEQUENCE, &[&name, &null])])
        }
        None => ber::tlv(ber::SEQUENCE, &[]),
    };
    let id = ber::integer(ber::INTEGER, i64::from(request_id));
    let zero = ber::integer(ber::INTEGER, 0);
    Some(ber::tlv(GET_REQUEST, &[&id, &zero, &zero, &bindings]))
}

/// The value of a variable binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::checks) enum Value {
    Integer(i64),
    Octets(Vec<u8>),
    Oid(Vec<u32>),
    IpAddress([u8; 4]),
    /// Counter32, Gauge32, `TimeTicks` and Counter64.
    Unsigned(u64),
    Opaque(Vec<u8>),
    Null,
    NoSuchObject,
    NoSuchInstance,
    EndOfMibView,
}

impl Value {
    fn read(tlv: &Tlv<'_>) -> Result<Self, Malformed> {
        Ok(match tlv.tag {
            ber::INTEGER => Self::Integer(tlv.integer()?),
            ber::OCTET_STRING => Self::Octets(tlv.content.to_vec()),
            ber::NULL if tlv.content.is_empty() => Self::Null,
            ber::OBJECT_IDENTIFIER => Self::Oid(tlv.oid()?),
            0x40 => Self::IpAddress(tlv.content.try_into().map_err(|_| Malformed)?),
            0x41..=0x43 | 0x46 => Self::Unsigned(tlv.unsigned()?),
            0x44 => Self::Opaque(tlv.content.to_vec()),
            0x80 if tlv.content.is_empty() => Self::NoSuchObject,
            0x81 if tlv.content.is_empty() => Self::NoSuchInstance,
            0x82 if tlv.content.is_empty() => Self::EndOfMibView,
            _ => return Err(Malformed),
        })
    }

    /// The text an expected value is compared with: integers in decimal, an OCTET STRING as
    /// UTF-8 or else as hex pairs (`00 1A FF`), an OID dotted, an `IpAddress` as a dotted quad.
    /// `None` when the binding holds no value.
    #[must_use]
    pub(in crate::checks) fn text(&self) -> Option<String> {
        Some(match self {
            Self::Integer(number) => number.to_string(),
            Self::Unsigned(number) => number.to_string(),
            Self::Octets(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => hex(bytes),
            },
            Self::Opaque(bytes) => hex(bytes),
            Self::Oid(arcs) => dotted(arcs),
            Self::IpAddress([a, b, c, d]) => format!("{a}.{b}.{c}.{d}"),
            Self::Null | Self::NoSuchObject | Self::NoSuchInstance | Self::EndOfMibView => {
                return None;
            }
        })
    }

    /// The fixed word for a binding that holds no value.
    const fn absence(&self) -> Option<&'static str> {
        match self {
            Self::Null => Some("null"),
            Self::NoSuchObject => Some("noSuchObject"),
            Self::NoSuchInstance => Some("noSuchInstance"),
            Self::EndOfMibView => Some("endOfMibView"),
            _ => None,
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 3);
    for (index, byte) in bytes.iter().enumerate() {
        let gap = if index == 0 { "" } else { " " };
        let _ = write!(text, "{gap}{byte:02X}");
    }
    text
}

/// An OID as dotted numbers.
#[must_use]
pub(in crate::checks) fn dotted(arcs: &[u32]) -> String {
    let parts: Vec<String> = arcs.iter().map(u32::to_string).collect();
    parts.join(".")
}

/// A PDU as read: its tag, its numbers and its bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::checks) struct Pdu {
    pub(in crate::checks) tag: u8,
    pub(in crate::checks) request_id: i64,
    pub(in crate::checks) error_status: i64,
    pub(in crate::checks) bindings: Vec<(Vec<u32>, Value)>,
}

impl Pdu {
    /// Reads a Response or Report PDU.
    ///
    /// # Errors
    /// Another PDU type, a field out of place, or more than eight bindings.
    pub(in crate::checks) fn read(tlv: &Tlv<'_>) -> Result<Self, Malformed> {
        if !matches!(tlv.tag, RESPONSE | REPORT) {
            return Err(Malformed);
        }
        let mut fields = tlv.reader();
        let request_id = fields.integer()?;
        let error_status = fields.integer()?;
        fields.integer()?;
        let mut list = fields.expect(ber::SEQUENCE)?.reader();
        fields.finish()?;
        let mut bindings = Vec::new();
        while !list.is_empty() {
            if bindings.len() == BINDINGS_MAX {
                return Err(Malformed);
            }
            let mut binding = list.expect(ber::SEQUENCE)?.reader();
            let name = binding.expect(ber::OBJECT_IDENTIFIER)?.oid()?;
            let value = Value::read(&binding.next()?)?;
            binding.finish()?;
            bindings.push((name, value));
        }
        Ok(Self {
            tag: tlv.tag,
            request_id,
            error_status,
            bindings,
        })
    }

    /// Reads the PDU that is the only value of `reader`'s rest.
    ///
    /// # Errors
    /// See [`Pdu::read`]; bytes after the PDU.
    pub(in crate::checks) fn last(reader: &mut Reader<'_>) -> Result<Self, Malformed> {
        let pdu = Self::read(&reader.next()?)?;
        reader.finish()?;
        Ok(pdu)
    }
}

/// The RFC 3416 name of an error status.
const fn error_name(status: i64) -> &'static str {
    match status {
        1 => "tooBig",
        2 => "noSuchName",
        3 => "badValue",
        4 => "readOnly",
        5 => "genErr",
        6 => "noAccess",
        7 => "wrongType",
        8 => "wrongLength",
        9 => "wrongEncoding",
        10 => "wrongValue",
        11 => "noCreation",
        12 => "inconsistentValue",
        13 => "resourceUnavailable",
        14 => "commitFailed",
        15 => "undoFailed",
        16 => "authorizationError",
        17 => "notWritable",
        18 => "inconsistentName",
        _ => "unknown",
    }
}

/// Judges a Response to the `GetRequest` for `oid`: no error, one binding for that OID, and a
/// value; with `expected`, a value whose text form equals it. Messages are fixed (LOG-007).
///
/// # Errors
/// The fixed failure the response gives.
pub(in crate::checks) fn judge(
    pdu: &Pdu,
    oid: &[u32],
    expected: Option<&str>,
) -> Result<(), Failure> {
    if pdu.error_status != 0 {
        return Err(Failure::SnmpError(error_name(pdu.error_status)));
    }
    let [(name, value)] = pdu.bindings.as_slice() else {
        return Err(Failure::SnmpMalformed);
    };
    if name.as_slice() != oid {
        return Err(Failure::SnmpMalformed);
    }
    if let Some(word) = value.absence() {
        return Err(Failure::SnmpNoValue(word));
    }
    match expected {
        Some(expected) if value.text().as_deref() != Some(expected) => Err(Failure::SnmpMismatch),
        _ => Ok(()),
    }
}
