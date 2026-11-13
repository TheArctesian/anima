//! The BER subset SNMP uses (X.690): single-byte tags, definite lengths, integers, octet
//! strings and object identifiers. Pure.
//!
//! The reader never trusts a length: each one is read in at most four bytes and must fit in
//! what is left of its parent, so a value can never reach past the datagram. Readers do not
//! recurse; the SNMP message grammar is walked level by level by the caller, so the nesting
//! depth is the grammar's own (at most six levels), whatever the input says. Nothing here
//! panics on any input.
//!
//! The writer builds into zeroized buffers sized up front, so a buffer that held a community
//! never grows and leaves no copy behind.

use zeroize::Zeroizing;

use crate::fleet::checks::config::OID_ARCS_MAX;

/// An encoded value whose buffer is wiped on drop.
pub(in crate::fleet::checks) type Bytes = Zeroizing<Vec<u8>>;

pub(in crate::fleet::checks) const INTEGER: u8 = 0x02;
pub(in crate::fleet::checks) const OCTET_STRING: u8 = 0x04;
pub(in crate::fleet::checks) const NULL: u8 = 0x05;
pub(in crate::fleet::checks) const OBJECT_IDENTIFIER: u8 = 0x06;
pub(in crate::fleet::checks) const SEQUENCE: u8 = 0x30;

/// The input is not the BER the grammar expects. It holds nothing of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::fleet::checks) struct Malformed;

/// One value read from the input: its tag, its content and where the content starts in the
/// whole input.
#[derive(Clone, Copy, Debug)]
pub(in crate::fleet::checks) struct Tlv<'a> {
    pub(in crate::fleet::checks) tag: u8,
    pub(in crate::fleet::checks) content: &'a [u8],
    /// The offset of `content` in the input the first reader was made from.
    pub(in crate::fleet::checks) offset: usize,
}

/// Reads the values one after the other inside one parent.
#[derive(Clone, Debug)]
pub(in crate::fleet::checks) struct Reader<'a> {
    rest: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub(in crate::fleet::checks) const fn new(input: &'a [u8]) -> Self {
        Self {
            rest: input,
            offset: 0,
        }
    }

    #[must_use]
    pub(in crate::fleet::checks) const fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// The next value.
    ///
    /// # Errors
    /// A multi-byte tag, an indefinite or oversize length, or a length past the parent.
    pub(in crate::fleet::checks) fn next(&mut self) -> Result<Tlv<'a>, Malformed> {
        let (&tag, after_tag) = self.rest.split_first().ok_or(Malformed)?;
        if tag & 0x1f == 0x1f {
            return Err(Malformed);
        }
        let (&first, mut after_length) = after_tag.split_first().ok_or(Malformed)?;
        let length = if first < 0x80 {
            usize::from(first)
        } else {
            let count = usize::from(first & 0x7f);
            if count == 0 || count > 4 || count > after_length.len() {
                return Err(Malformed);
            }
            let (digits, rest) = after_length.split_at(count);
            after_length = rest;
            digits
                .iter()
                .fold(0_usize, |sum, digit| (sum << 8) | usize::from(*digit))
        };
        if length > after_length.len() {
            return Err(Malformed);
        }
        let header = self.rest.len() - after_length.len();
        let (content, rest) = after_length.split_at(length);
        let tlv = Tlv {
            tag,
            content,
            offset: self.offset + header,
        };
        self.offset += header + length;
        self.rest = rest;
        Ok(tlv)
    }

    /// The next value, which must have this tag.
    ///
    /// # Errors
    /// The value is malformed or has another tag.
    pub(in crate::fleet::checks) fn expect(&mut self, tag: u8) -> Result<Tlv<'a>, Malformed> {
        let tlv = self.next()?;
        if tlv.tag == tag {
            Ok(tlv)
        } else {
            Err(Malformed)
        }
    }

    /// The content of the next value, which must be an INTEGER that fits an `i64`.
    ///
    /// # Errors
    /// The value is malformed, has another tag or does not fit.
    pub(in crate::fleet::checks) fn integer(&mut self) -> Result<i64, Malformed> {
        self.expect(INTEGER)?.integer()
    }

    /// The content of the next value, which must be an OCTET STRING.
    ///
    /// # Errors
    /// The value is malformed or has another tag.
    pub(in crate::fleet::checks) fn octets(&mut self) -> Result<&'a [u8], Malformed> {
        Ok(self.expect(OCTET_STRING)?.content)
    }

    /// Fails unless every value of the parent was read.
    ///
    /// # Errors
    /// Bytes are left over.
    pub(in crate::fleet::checks) const fn finish(&self) -> Result<(), Malformed> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(Malformed)
        }
    }
}

impl<'a> Tlv<'a> {
    /// A reader over the content, for a constructed value.
    #[must_use]
    pub(in crate::fleet::checks) const fn reader(&self) -> Reader<'a> {
        Reader {
            rest: self.content,
            offset: self.offset,
        }
    }

    /// The content as a two's complement integer of one to eight bytes.
    ///
    /// # Errors
    /// The content is empty or longer than eight bytes.
    pub(in crate::fleet::checks) fn integer(&self) -> Result<i64, Malformed> {
        let (&first, _) = self.content.split_first().ok_or(Malformed)?;
        if self.content.len() > 8 {
            return Err(Malformed);
        }
        let start = if first & 0x80 == 0 { 0 } else { -1 };
        Ok(self
            .content
            .iter()
            .fold(start, |sum: i64, byte| (sum << 8) | i64::from(*byte)))
    }

    /// The content as an unsigned integer (Counter32, Gauge32, `TimeTicks`, Counter64): one
    /// leading zero byte is allowed, then at most eight bytes.
    ///
    /// # Errors
    /// The content is empty or too long.
    pub(in crate::fleet::checks) fn unsigned(&self) -> Result<u64, Malformed> {
        let digits = match self.content {
            [0, rest @ ..] if !rest.is_empty() => rest,
            other => other,
        };
        if digits.is_empty() || digits.len() > 8 {
            return Err(Malformed);
        }
        Ok(digits
            .iter()
            .fold(0_u64, |sum, byte| (sum << 8) | u64::from(*byte)))
    }

    /// The content as the arcs of an object identifier.
    ///
    /// # Errors
    /// An empty content, an unfinished arc, an arc over `u32` or more than
    /// [`OID_ARCS_MAX`] arcs.
    pub(in crate::fleet::checks) fn oid(&self) -> Result<Vec<u32>, Malformed> {
        let mut arcs = Vec::new();
        let mut value: u64 = 0;
        let mut pending = false;
        for byte in self.content {
            value = (value << 7) | u64::from(byte & 0x7f);
            if value > u64::from(u32::MAX) + 80 || (!pending && *byte == 0x80) {
                return Err(Malformed);
            }
            pending = byte & 0x80 != 0;
            if pending {
                continue;
            }
            if arcs.is_empty() {
                let (first, second) = match value {
                    0..40 => (0, value),
                    40..80 => (1, value - 40),
                    _ => (2, value - 80),
                };
                arcs.push(u32::try_from(first).map_err(|_| Malformed)?);
                arcs.push(u32::try_from(second).map_err(|_| Malformed)?);
            } else {
                arcs.push(u32::try_from(value).map_err(|_| Malformed)?);
            }
            if arcs.len() > OID_ARCS_MAX {
                return Err(Malformed);
            }
            value = 0;
        }
        if pending || arcs.is_empty() {
            return Err(Malformed);
        }
        Ok(arcs)
    }
}

/// The length octets of a definite length.
fn length(length: usize) -> Vec<u8> {
    if length < 0x80 {
        // A short-form length is one byte below 0x80.
        return vec![u8::try_from(length).unwrap_or(0)];
    }
    let bytes = length.to_be_bytes();
    let skip = bytes.iter().take_while(|b| **b == 0).count();
    let count = u8::try_from(bytes.len() - skip).unwrap_or(0);
    let mut out = vec![0x80 | count];
    out.extend_from_slice(&bytes[skip..]);
    out
}

/// One value with this tag around the concatenated parts.
#[must_use]
pub(in crate::fleet::checks) fn tlv(tag: u8, parts: &[&[u8]]) -> Bytes {
    let size: usize = parts.iter().map(|part| part.len()).sum();
    let length = length(size);
    let mut out = Zeroizing::new(Vec::with_capacity(1 + length.len() + size));
    out.push(tag);
    out.extend_from_slice(&length);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

/// An INTEGER (or an application integer type) in its shortest two's complement form.
#[must_use]
pub(in crate::fleet::checks) fn integer(tag: u8, value: i64) -> Bytes {
    let bytes = value.to_be_bytes();
    let mut skip = 0;
    while skip < 7 {
        let (byte, next) = (bytes[skip], bytes[skip + 1]);
        let redundant = (byte == 0 && next & 0x80 == 0) || (byte == 0xff && next & 0x80 != 0);
        if !redundant {
            break;
        }
        skip += 1;
    }
    tlv(tag, &[&bytes[skip..]])
}

/// An OCTET STRING.
#[must_use]
pub(in crate::fleet::checks) fn octets(bytes: &[u8]) -> Bytes {
    tlv(OCTET_STRING, &[bytes])
}

/// An OBJECT IDENTIFIER; `None` when the first two arcs cannot be encoded.
#[must_use]
pub(in crate::fleet::checks) fn oid(arcs: &[u32]) -> Option<Bytes> {
    let [first, second, rest @ ..] = arcs else {
        return None;
    };
    if *first > 2 || (*first < 2 && *second >= 40) {
        return None;
    }
    let head = u64::from(*first) * 40 + u64::from(*second);
    let mut content = Vec::with_capacity(arcs.len() * 5);
    for arc in std::iter::once(head).chain(rest.iter().map(|arc| u64::from(*arc))) {
        let mut groups = vec![u8::try_from(arc & 0x7f).unwrap_or(0)];
        let mut left = arc >> 7;
        while left > 0 {
            groups.push(0x80 | u8::try_from(left & 0x7f).unwrap_or(0));
            left >>= 7;
        }
        content.extend(groups.iter().rev());
    }
    Some(tlv(OBJECT_IDENTIFIER, &[&content]))
}
