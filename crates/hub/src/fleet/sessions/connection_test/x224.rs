//! The X.224 exchange that opens every RDP connection (MS-RDPBCGR 2.2.1.1 and 2.2.1.2), as
//! bytes: the Connection Request with an RDP Negotiation Request the hub sends, and the
//! Connection Confirm it reads back. Pure. A length from the server is checked before it is
//! used, and a reply that does not fit is [`Malformed`], never a panic.

/// Standard RDP security: no TLS, so no certificate.
pub(in crate::fleet::sessions) const PROTOCOL_RDP: u32 = 0;
pub(in crate::fleet::sessions) const PROTOCOL_SSL: u32 = 0x1;
pub(in crate::fleet::sessions) const PROTOCOL_HYBRID: u32 = 0x2;
pub(in crate::fleet::sessions) const PROTOCOL_HYBRID_EX: u32 = 0x8;
/// Every protocol that starts with a TLS handshake and that the hub asks for: TLS, NLA (`CredSSP`
/// over TLS) and NLA with the early authorisation result.
pub(in crate::fleet::sessions) const TLS_PROTOCOLS: u32 =
    PROTOCOL_SSL | PROTOCOL_HYBRID | PROTOCOL_HYBRID_EX;

/// The TPKT header (RFC 1006): version, reserved, then the whole packet's length.
pub(in crate::fleet::sessions) const TPKT_HEADER: usize = 4;
const TPKT_VERSION: u8 = 3;
/// Longest confirm the hub reads, header included. A real one is 19 bytes (11 without
/// negotiation data); this leaves room for X.224 user data the hub ignores.
pub(in crate::fleet::sessions) const MAX_CONFIRM: usize = 64;
const REQUEST_LENGTH: usize = 19;

/// TPDU codes in the high nibble of the byte after the length indicator.
const CONNECTION_REQUEST: u8 = 0xe0;
const CONNECTION_CONFIRM: u8 = 0xd0;
/// The fixed part of a CR or CC TPDU after the length indicator: code, destination
/// reference (2), source reference (2), class.
const FIXED_PART: u8 = 6;

const NEGOTIATION_REQUEST: u8 = 0x01;
const NEGOTIATION_RESPONSE: u8 = 0x02;
const NEGOTIATION_FAILURE: u8 = 0x03;
/// Every negotiation structure is 8 bytes: type, flags, length (2), value (4).
const NEGOTIATION_LENGTH: u8 = 8;

/// What the server chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::fleet::sessions) enum Negotiated {
    /// One of the requested TLS protocols: a TLS handshake comes next.
    Tls(u32),
    /// Standard RDP security only, chosen or the only one an older server knows.
    Standard,
    /// An RDP Negotiation Failure with its code (for example `SSL_NOT_ALLOWED_BY_SERVER`).
    Failure(u32),
}

/// The reply is not an X.224 Connection Confirm the hub can read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the reply is not an X.224 connection confirm")]
pub(in crate::fleet::sessions) struct Malformed;

/// The Connection Request asking for `requested` (a set of the `PROTOCOL_*` flags), with no
/// routing token or cookie: the hub names no user.
pub(in crate::fleet::sessions) fn connection_request(requested: u32) -> [u8; REQUEST_LENGTH] {
    let mut packet = [0; REQUEST_LENGTH];
    let total = u16::try_from(REQUEST_LENGTH)
        .unwrap_or(u16::MAX)
        .to_be_bytes();
    packet[..TPKT_HEADER].copy_from_slice(&[TPKT_VERSION, 0, total[0], total[1]]);
    // The length indicator counts the TPDU header after itself.
    packet[4] = FIXED_PART + NEGOTIATION_LENGTH;
    packet[5] = CONNECTION_REQUEST;
    // Destination and source references and the class stay zero.
    let negotiation = &mut packet[TPKT_HEADER + 1 + usize::from(FIXED_PART)..];
    negotiation[0] = NEGOTIATION_REQUEST;
    negotiation[2..4].copy_from_slice(&u16::from(NEGOTIATION_LENGTH).to_le_bytes());
    negotiation[4..8].copy_from_slice(&requested.to_le_bytes());
    packet
}

/// How many bytes of the confirm follow its TPKT `header`, when the header is version 3 and
/// the packet is long enough for a confirm and at most [`MAX_CONFIRM`].
///
/// # Errors
/// [`Malformed`] for any other header.
pub(in crate::fleet::sessions) fn confirm_length(
    header: [u8; TPKT_HEADER],
) -> Result<usize, Malformed> {
    let [version, _reserved, high, low] = header;
    let total = usize::from(u16::from_be_bytes([high, low]));
    let fits = (TPKT_HEADER + 1 + usize::from(FIXED_PART)..=MAX_CONFIRM).contains(&total);
    if version == TPKT_VERSION && fits {
        Ok(total - TPKT_HEADER)
    } else {
        Err(Malformed)
    }
}

/// What the server chose, from the confirm's bytes after the TPKT header, given what the hub
/// `requested`. A choice the hub did not ask for is [`Malformed`].
///
/// # Errors
/// [`Malformed`] when the bytes are not a Connection Confirm with a valid negotiation part.
pub(in crate::fleet::sessions) fn negotiated(
    body: &[u8],
    requested: u32,
) -> Result<Negotiated, Malformed> {
    let (&indicator, rest) = body.split_first().ok_or(Malformed)?;
    let header = rest.get(..usize::from(indicator)).ok_or(Malformed)?;
    let code = header.first().ok_or(Malformed)?;
    if code & 0xf0 != CONNECTION_CONFIRM {
        return Err(Malformed);
    }
    match header.get(usize::from(FIXED_PART)..) {
        Some([]) => Ok(Negotiated::Standard),
        Some(negotiation) => negotiation_reply(negotiation, requested),
        None => Err(Malformed),
    }
}

/// The RDP Negotiation Response or Failure at the start of `bytes`.
fn negotiation_reply(bytes: &[u8], requested: u32) -> Result<Negotiated, Malformed> {
    let Some(&[kind, _flags, length_low, length_high, a, b, c, d]) =
        bytes.get(..usize::from(NEGOTIATION_LENGTH))
    else {
        return Err(Malformed);
    };
    if u16::from_le_bytes([length_low, length_high]) != u16::from(NEGOTIATION_LENGTH) {
        return Err(Malformed);
    }
    let value = u32::from_le_bytes([a, b, c, d]);
    match kind {
        NEGOTIATION_RESPONSE if value == PROTOCOL_RDP => Ok(Negotiated::Standard),
        NEGOTIATION_RESPONSE if value & !requested == 0 && value & TLS_PROTOCOLS != 0 => {
            Ok(Negotiated::Tls(value))
        }
        NEGOTIATION_FAILURE => Ok(Negotiated::Failure(value)),
        _ => Err(Malformed),
    }
}
