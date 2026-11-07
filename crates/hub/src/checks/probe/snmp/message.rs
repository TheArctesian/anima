//! The SNMP v3 message with the user-based security model (RFC 3412 Section 6, RFC 3414
//! Section 2.4): engine discovery, the authPriv `GetRequest`, and reading the answers. Pure:
//! ids and the salt come in as parameters.
//!
//! Nothing in an answer is trusted before its digest is verified, except a Report that is
//! not authenticated, which only ever ends the attempt as Down with a fixed message.

use zeroize::Zeroizing;

use super::{
    ber::{self, Bytes, Malformed, Reader, Tlv},
    pdu::{self, Pdu},
    usm::{self, DIGEST_LEN, Keys, SALT_LEN},
};
use crate::checks::probe::Failure;

const VERSION: i64 = 3;
const USM: i64 = 3;
const AUTH: u8 = 0x01;
const PRIV: u8 = 0x02;
const REPORTABLE: u8 = 0x04;
/// The largest message the hub accepts, which it advertises as `msgMaxSize`.
pub(in crate::checks) const DATAGRAM_MAX: usize = 65_507;
/// `snmpEngineID` is 5 to 32 bytes (RFC 3411).
const ENGINE_ID: std::ops::RangeInclusive<usize> = 5..=32;

/// The ids of one request: `msgID` and the PDU's request-id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::checks) struct Ids {
    pub(in crate::checks) message: i32,
    pub(in crate::checks) request: i32,
}

/// The agent's engine, as discovery reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::checks) struct Engine {
    pub(in crate::checks) id: Vec<u8>,
    pub(in crate::checks) boots: u32,
    pub(in crate::checks) time: u32,
}

/// A message as read, before anything in it is trusted.
struct Parsed<'a> {
    message_id: i64,
    flags: u8,
    engine_id: &'a [u8],
    boots: i64,
    time: i64,
    user: &'a [u8],
    digest: Tlv<'a>,
    salt: &'a [u8],
    scoped: Tlv<'a>,
}

fn parse(datagram: &[u8]) -> Result<Parsed<'_>, Malformed> {
    let mut outer = Reader::new(datagram);
    let mut message = outer.expect(ber::SEQUENCE)?.reader();
    outer.finish()?;
    if message.integer()? != VERSION {
        return Err(Malformed);
    }
    let mut header = message.expect(ber::SEQUENCE)?.reader();
    let message_id = header.integer()?;
    header.integer()?;
    let &[flags] = header.octets()? else {
        return Err(Malformed);
    };
    if header.integer()? != USM {
        return Err(Malformed);
    }
    header.finish()?;
    let mut security = message.expect(ber::OCTET_STRING)?.reader();
    let mut fields = security.expect(ber::SEQUENCE)?.reader();
    security.finish()?;
    let parsed = Parsed {
        message_id,
        flags,
        engine_id: fields.octets()?,
        boots: fields.integer()?,
        time: fields.integer()?,
        user: fields.octets()?,
        digest: fields.expect(ber::OCTET_STRING)?,
        salt: fields.octets()?,
        scoped: message.next()?,
    };
    fields.finish()?;
    message.finish()?;
    Ok(parsed)
}

fn security(engine: &Engine, user: &[u8], digest: &[u8], salt: &[u8]) -> Bytes {
    let fields = [
        ber::octets(&engine.id),
        ber::integer(ber::INTEGER, i64::from(engine.boots)),
        ber::integer(ber::INTEGER, i64::from(engine.time)),
        ber::octets(user),
        ber::octets(digest),
        ber::octets(salt),
    ];
    let parts: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
    ber::octets(&ber::tlv(ber::SEQUENCE, &parts))
}

fn message(message_id: i32, flags: u8, security: &[u8], scoped: &[u8]) -> Bytes {
    let size = i64::try_from(DATAGRAM_MAX).unwrap_or(i64::MAX);
    let header = ber::tlv(
        ber::SEQUENCE,
        &[
            &ber::integer(ber::INTEGER, i64::from(message_id)),
            &ber::integer(ber::INTEGER, size),
            &ber::octets(&[flags]),
            &ber::integer(ber::INTEGER, USM),
        ],
    );
    let version = ber::integer(ber::INTEGER, VERSION);
    ber::tlv(ber::SEQUENCE, &[&version, &header, security, scoped])
}

fn scoped(context_engine: &[u8], pdu: &[u8]) -> Bytes {
    ber::tlv(
        ber::SEQUENCE,
        &[&ber::octets(context_engine), &ber::octets(&[]), pdu],
    )
}

/// The discovery request: no user, no authentication, no binding, reportable.
#[must_use]
pub(in crate::checks) fn discovery(ids: Ids) -> Bytes {
    let unknown = Engine {
        id: Vec::new(),
        boots: 0,
        time: 0,
    };
    let pdu = pdu::get_request(ids.request, None).unwrap_or_default();
    let scoped = scoped(&[], &pdu);
    message(ids.message, REPORTABLE, &security(&unknown, &[], &[], &[]), &scoped)
}

fn engine_of(parsed: &Parsed<'_>) -> Result<Engine, Malformed> {
    if parsed.flags & (AUTH | PRIV) != 0 || !ENGINE_ID.contains(&parsed.engine_id.len()) {
        return Err(Malformed);
    }
    let pdu = Pdu::last(&mut parsed.scoped_plain()?)?;
    if pdu.tag != pdu::REPORT {
        return Err(Malformed);
    }
    Ok(Engine {
        id: parsed.engine_id.to_vec(),
        boots: counter(parsed.boots)?,
        time: counter(parsed.time)?,
    })
}

/// The engine the discovery Report names. `None` for a reply to another message.
#[must_use]
pub(in crate::checks) fn engine(datagram: &[u8], ids: Ids) -> Option<Result<Engine, Failure>> {
    let parsed = match parse(datagram) {
        Ok(parsed) => parsed,
        Err(Malformed) => return Some(Err(Failure::SnmpMalformed)),
    };
    if parsed.message_id != i64::from(ids.message) {
        return None;
    }
    Some(engine_of(&parsed).map_err(|Malformed| Failure::SnmpMalformed))
}

/// An engine boots or time value: 0 to 2^31 - 1.
fn counter(value: i64) -> Result<u32, Malformed> {
    u32::try_from(value)
        .ok()
        .filter(|value| i32::try_from(*value).is_ok())
        .ok_or(Malformed)
}

impl Parsed<'_> {
    fn scoped_plain(&self) -> Result<Reader<'_>, Malformed> {
        if self.scoped.tag == ber::SEQUENCE {
            Ok(self.scoped.reader())
        } else {
            Err(Malformed)
        }
    }
}

/// The authPriv `GetRequest` for `oid`: the scoped PDU encrypted with the salt, and the digest
/// over the whole message. `None` when the OID cannot be encoded.
#[must_use]
pub(in crate::checks) fn get(
    (keys, engine): (&Keys, &Engine),
    user: &[u8],
    oid: &[u32],
    (ids, salt): (Ids, [u8; SALT_LEN]),
) -> Option<Bytes> {
    let pdu = pdu::get_request(ids.request, Some(oid))?;
    let mut sealed = scoped(&engine.id, &pdu);
    usm::encrypt(&keys.privacy, usm::iv(engine.boots, engine.time, salt), &mut sealed);
    let security = security(engine, user, &[0; DIGEST_LEN], &salt);
    let flags = AUTH | PRIV | REPORTABLE;
    let mut whole = message(ids.message, flags, &security, &ber::octets(&sealed));
    let start = parse(&whole).ok()?.digest.offset;
    let digest = usm::digest(&keys.auth, &whole);
    whole.get_mut(start..start + DIGEST_LEN)?.copy_from_slice(&digest);
    Some(whole)
}

/// The fixed failure a Report gives, by the USM statistic it carries (RFC 3414 Section 5).
fn report(pdu: &Pdu) -> Failure {
    const USM_STATS: [u32; 9] = [1, 3, 6, 1, 6, 3, 15, 1, 1];
    let reason = match pdu.bindings.first().map(|(name, _)| name.as_slice()) {
        Some([prefix @ .., counter, 0]) if *prefix == USM_STATS => match counter {
            1 => "unsupported security level",
            2 => "not in time window",
            3 => "unknown user name",
            4 => "unknown engine ID",
            5 => "wrong digest",
            6 => "decryption error",
            _ => "an unexpected report",
        },
        _ => "an unexpected report",
    };
    Failure::SnmpReport(reason)
}

/// The digest is checked over a copy of the message whose digest field is zeroed.
fn authentic(datagram: &[u8], parsed: &Parsed<'_>, keys: &Keys) -> bool {
    let range = parsed.digest.offset..parsed.digest.offset + parsed.digest.content.len();
    let mut copy = Zeroizing::new(datagram.to_vec());
    copy.get_mut(range).map(|digest| digest.fill(0)).is_some()
        && usm::verify(&keys.auth, &copy, parsed.digest.content)
}

/// The scoped PDU of an authenticated message, decrypted when it is private.
fn scoped_pdu(parsed: &Parsed<'_>, keys: &Keys) -> Result<Pdu, Malformed> {
    if parsed.flags & PRIV == 0 {
        return read_scoped(&mut parsed.scoped_plain()?);
    }
    let salt: [u8; SALT_LEN] = parsed.salt.try_into().map_err(|_| Malformed)?;
    if parsed.scoped.tag != ber::OCTET_STRING {
        return Err(Malformed);
    }
    let iv = usm::iv(counter(parsed.boots)?, counter(parsed.time)?, salt);
    let mut plain = Zeroizing::new(parsed.scoped.content.to_vec());
    usm::decrypt(&keys.privacy, iv, &mut plain);
    // CFB needs no padding, but an agent may pad; bytes after the scoped PDU are ignored.
    let mut reader = Reader::new(&plain).expect(ber::SEQUENCE)?.reader();
    read_scoped(&mut reader)
}

fn read_scoped(reader: &mut Reader<'_>) -> Result<Pdu, Malformed> {
    reader.octets()?;
    reader.octets()?;
    Pdu::last(reader)
}

/// The Response to the authPriv `GetRequest`. `None` for a reply to another message. A
/// message that fails its digest, or a Response that is not authenticated, is never read.
#[must_use]
pub(in crate::checks) fn response(
    datagram: &[u8],
    (keys, engine): (&Keys, &Engine),
    user: &[u8],
    ids: Ids,
) -> Option<Result<Pdu, Failure>> {
    let Ok(parsed) = parse(datagram) else {
        return Some(Err(Failure::SnmpMalformed));
    };
    if parsed.message_id != i64::from(ids.message) {
        return None;
    }
    if parsed.flags & AUTH == 0 {
        let unauthenticated = parsed.flags & PRIV == 0;
        let pdu = parsed.scoped_plain().and_then(|mut r| read_scoped(&mut r));
        return Some(Err(match pdu {
            Ok(pdu) if unauthenticated && pdu.tag == pdu::REPORT => report(&pdu),
            Ok(_) => Failure::SnmpUnauthenticated,
            Err(Malformed) => Failure::SnmpMalformed,
        }));
    }
    if !authentic(datagram, &parsed, keys) {
        return Some(Err(Failure::SnmpAuthentication));
    }
    if parsed.engine_id != engine.id.as_slice() || parsed.user != user {
        return Some(Err(Failure::SnmpMalformed));
    }
    Some(match scoped_pdu(&parsed, keys) {
        Ok(pdu) if pdu.tag == pdu::REPORT => Err(report(&pdu)),
        Ok(pdu) if pdu.request_id == i64::from(ids.request) => Ok(pdu),
        Ok(_) | Err(Malformed) => Err(Failure::SnmpMalformed),
    })
}
