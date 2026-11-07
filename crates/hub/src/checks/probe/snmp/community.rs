//! The SNMP v2c message (RFC 1901): the community, then the PDU. Pure.

use super::{
    ber::{self, Bytes, Malformed, Reader},
    pdu::{self, Pdu},
};
use crate::checks::probe::Failure;

/// The version field of an SNMP v2c message.
const VERSION: i64 = 1;

/// A `GetRequest` for `oid` with the community. `None` when the OID cannot be encoded.
#[must_use]
pub(in crate::checks) fn request(community: &[u8], oid: &[u32], request_id: i32) -> Option<Bytes> {
    let pdu = pdu::get_request(request_id, Some(oid))?;
    let version = ber::integer(ber::INTEGER, VERSION);
    let community = ber::octets(community);
    Some(ber::tlv(ber::SEQUENCE, &[&version, &community, &pdu]))
}

fn read(datagram: &[u8]) -> Result<Pdu, Malformed> {
    let mut outer = Reader::new(datagram);
    let mut message = outer.expect(ber::SEQUENCE)?.reader();
    outer.finish()?;
    if message.integer()? != VERSION {
        return Err(Malformed);
    }
    message.octets()?;
    Pdu::last(&mut message)
}

/// The Response to the request with this id. `None` for a reply to another request, which
/// the caller ignores; a datagram that is not an SNMP v2c Response is malformed.
#[must_use]
pub(in crate::checks) fn response(
    datagram: &[u8],
    request_id: i32,
) -> Option<Result<Pdu, Failure>> {
    match read(datagram) {
        Ok(pdu) if pdu.request_id != i64::from(request_id) => None,
        Ok(pdu) if pdu.tag == pdu::RESPONSE => Some(Ok(pdu)),
        Ok(_) | Err(Malformed) => Some(Err(Failure::SnmpMalformed)),
    }
}
