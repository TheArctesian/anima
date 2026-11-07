//! A local SNMP agent on 127.0.0.1 for the probe tests: v2c with one community and v3
//! authPriv with one user. Its message handling is written here from RFC 3412, RFC 3414 and
//! RFC 3416; it shares only the BER writer and the USM primitives that the vectors pin.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tokio::net::UdpSocket;
use zeroize::Zeroizing;

use crate::checks::probe::snmp::{
    ber::{self, Reader, Tlv},
    pdu,
    usm::{self, Keys},
};

pub const ENGINE_ID: [u8; 9] = [0x80, 0, 0x1f, 0x88, 0x80, 1, 2, 3, 4];
pub const BOOTS: u32 = 3;
pub const TIME: u32 = 1234;
const AGENT_SALT: [u8; 8] = [0xa5; 8];

/// How the agent behaves.
pub struct Setup {
    pub community: &'static str,
    pub user: &'static str,
    pub auth: &'static str,
    pub privacy: &'static str,
    /// The OIDs it knows, with their encoded values.
    pub values: Vec<(Vec<u32>, Vec<u8>)>,
    /// It reads every request and answers none.
    pub silent: bool,
    /// Before each answer, a datagram from another port that would fail the probe if read.
    pub stray_first: bool,
    /// It flips a byte of the digest of each authenticated answer.
    pub tamper: bool,
}

impl Setup {
    pub fn new() -> Self {
        Self {
            community: "c0mmunity-SECRET-v2c",
            user: "monitor",
            auth: "auth-SECRET-password",
            privacy: "privacy-SECRET-password",
            values: vec![
                (
                    vec![1, 3, 6, 1, 2, 1, 1, 5, 0],
                    ber::octets(b"nas-01").to_vec(),
                ),
                (
                    vec![1, 3, 6, 1, 2, 1, 1, 3, 0],
                    ber::integer(0x43, 12_345).to_vec(),
                ),
            ],
            silent: false,
            stray_first: false,
            tamper: false,
        }
    }
}

pub struct Agent {
    pub port: u16,
    pub requests: Arc<AtomicUsize>,
}

/// Starts the agent on an ephemeral loopback port.
pub async fn spawn(setup: Setup) -> Agent {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let stray = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let agent = Agent {
        port: socket.local_addr().unwrap().port(),
        requests: Arc::default(),
    };
    let requests = agent.requests.clone();
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 65_535];
        while let Ok((length, from)) = socket.recv_from(&mut buffer).await {
            requests.fetch_add(1, Ordering::SeqCst);
            if setup.silent {
                continue;
            }
            let Some(reply) = answer(&setup, &buffer[..length]) else {
                continue;
            };
            if setup.stray_first {
                let _ = stray.send_to(&[0x30, 0x03, 0x02, 0x01, 0x01], from).await;
            }
            let _ = socket.send_to(&reply, from).await;
        }
    });
    agent
}

fn answer(setup: &Setup, datagram: &[u8]) -> Option<Vec<u8>> {
    let mut message = Reader::new(datagram).expect(ber::SEQUENCE).ok()?.reader();
    match message.integer().ok()? {
        1 => v2c(setup, &mut message),
        3 => v3(setup, datagram, &mut message),
        _ => None,
    }
}

/// The Response PDU to a `GetRequest` PDU.
fn respond(setup: &Setup, request: &Tlv<'_>) -> Option<Vec<u8>> {
    let mut fields = request.reader();
    let request_id = fields.integer().ok()?;
    fields.integer().ok()?;
    fields.integer().ok()?;
    let mut list = fields.expect(ber::SEQUENCE).ok()?.reader();
    let mut binding = list.expect(ber::SEQUENCE).ok()?.reader();
    let name = binding.expect(ber::OBJECT_IDENTIFIER).ok()?.oid().ok()?;
    let value = setup
        .values
        .iter()
        .find(|(known, _)| *known == name)
        .map_or_else(|| vec![0x80, 0], |(_, value)| value.clone());
    let binding = ber::tlv(ber::SEQUENCE, &[&ber::oid(&name)?, &value]);
    let list = ber::tlv(ber::SEQUENCE, &[&binding]);
    let numbers = [request_id, 0, 0].map(|n| ber::integer(ber::INTEGER, n));
    Some(
        ber::tlv(
            pdu::RESPONSE,
            &[&numbers[0], &numbers[1], &numbers[2], &list],
        )
        .to_vec(),
    )
}

fn v2c(setup: &Setup, message: &mut Reader<'_>) -> Option<Vec<u8>> {
    // A wrong community gets no answer, as real agents do.
    if message.octets().ok()? != setup.community.as_bytes() {
        return None;
    }
    let pdu = respond(setup, &message.next().ok()?)?;
    let version = ber::integer(ber::INTEGER, 1);
    let community = ber::octets(setup.community.as_bytes());
    Some(ber::tlv(ber::SEQUENCE, &[&version, &community, &pdu]).to_vec())
}

/// The fields of a v3 request the agent uses.
struct Request<'a> {
    id: i64,
    flags: u8,
    boots: i64,
    time: i64,
    user: &'a [u8],
    digest: Tlv<'a>,
    salt: &'a [u8],
    scoped: Tlv<'a>,
}

fn request<'a>(message: &mut Reader<'a>) -> Option<Request<'a>> {
    let mut header = message.expect(ber::SEQUENCE).ok()?.reader();
    let id = header.integer().ok()?;
    header.integer().ok()?;
    let flags = *header.octets().ok()?.first()?;
    let security = message.expect(ber::OCTET_STRING).ok()?;
    let mut fields = security.reader().expect(ber::SEQUENCE).ok()?.reader();
    fields.octets().ok()?;
    Some(Request {
        id,
        flags,
        boots: fields.integer().ok()?,
        time: fields.integer().ok()?,
        user: fields.octets().ok()?,
        digest: fields.expect(ber::OCTET_STRING).ok()?,
        salt: fields.octets().ok()?,
        scoped: message.next().ok()?,
    })
}

fn keys(setup: &Setup) -> Keys {
    Keys::localised(setup.auth.as_bytes(), setup.privacy.as_bytes(), &ENGINE_ID).unwrap()
}

fn v3(setup: &Setup, datagram: &[u8], message: &mut Reader<'_>) -> Option<Vec<u8>> {
    let request = request(message)?;
    if request.flags & 0x01 == 0 {
        return Some(report(request.id, 4));
    }
    if request.user != setup.user.as_bytes() {
        return Some(report(request.id, 3));
    }
    let keys = keys(setup);
    let at = request.digest.offset;
    let mut zeroed = datagram.to_vec();
    zeroed[at..at + request.digest.content.len()].fill(0);
    if !usm::verify(&keys.auth, &zeroed, request.digest.content) {
        return Some(report(request.id, 5));
    }
    let salt: [u8; 8] = request.salt.try_into().ok()?;
    let boots = u32::try_from(request.boots).ok()?;
    let time = u32::try_from(request.time).ok()?;
    let mut plain = Zeroizing::new(request.scoped.content.to_vec());
    usm::decrypt(&keys.privacy, usm::iv(boots, time, salt), &mut plain);
    let mut scoped = Reader::new(&plain).expect(ber::SEQUENCE).ok()?.reader();
    scoped.octets().ok()?;
    scoped.octets().ok()?;
    let pdu = respond(setup, &scoped.next().ok()?)?;
    let mut reply = private(setup, &keys, request.id, &pdu);
    if setup.tamper {
        let at = digest_offset(&reply);
        reply[at] ^= 1;
    }
    Some(reply)
}

fn security(user: &[u8], digest: &[u8], salt: &[u8]) -> Vec<u8> {
    let fields = [
        ber::octets(&ENGINE_ID),
        ber::integer(ber::INTEGER, i64::from(BOOTS)),
        ber::integer(ber::INTEGER, i64::from(TIME)),
        ber::octets(user),
        ber::octets(digest),
        ber::octets(salt),
    ];
    let parts: Vec<&[u8]> = fields.iter().map(|f| f.as_slice()).collect();
    ber::octets(&ber::tlv(ber::SEQUENCE, &parts)).to_vec()
}

fn whole(id: i64, flags: u8, security: &[u8], scoped: &[u8]) -> Vec<u8> {
    let header = [
        ber::integer(ber::INTEGER, id),
        ber::integer(ber::INTEGER, 65_507),
        ber::octets(&[flags]),
        ber::integer(ber::INTEGER, 3),
    ];
    let parts: Vec<&[u8]> = header.iter().map(|f| f.as_slice()).collect();
    let header = ber::tlv(ber::SEQUENCE, &parts);
    let version = ber::integer(ber::INTEGER, 3);
    ber::tlv(ber::SEQUENCE, &[&version, &header, security, scoped]).to_vec()
}

fn scoped(pdu: &[u8]) -> Vec<u8> {
    ber::tlv(
        ber::SEQUENCE,
        &[&ber::octets(&ENGINE_ID), &ber::octets(&[]), pdu],
    )
    .to_vec()
}

/// An unauthenticated Report of the USM statistic `counter` (RFC 3414 Section 3.2).
fn report(id: i64, counter: u32) -> Vec<u8> {
    let name = ber::oid(&[1, 3, 6, 1, 6, 3, 15, 1, 1, counter, 0]).unwrap();
    let binding = ber::tlv(ber::SEQUENCE, &[&name, &ber::integer(0x41, 1)]);
    let list = ber::tlv(ber::SEQUENCE, &[&binding]);
    let numbers = [0, 0, 0].map(|n| ber::integer(ber::INTEGER, n));
    let pdu = ber::tlv(pdu::REPORT, &[&numbers[0], &numbers[1], &numbers[2], &list]);
    whole(id, 0, &security(&[], &[], &[]), &scoped(&pdu))
}

/// Where the digest of a message the agent built starts.
fn digest_offset(message: &[u8]) -> usize {
    let mut fields = Reader::new(message).expect(ber::SEQUENCE).unwrap().reader();
    fields.integer().unwrap();
    fields.expect(ber::SEQUENCE).unwrap();
    let security = fields.expect(ber::OCTET_STRING).unwrap();
    let mut inner = security.reader().expect(ber::SEQUENCE).unwrap().reader();
    for _ in 0..4 {
        inner.next().unwrap();
    }
    inner.expect(ber::OCTET_STRING).unwrap().offset
}

/// An authPriv Response: encrypted with the agent's salt and signed.
fn private(setup: &Setup, keys: &Keys, id: i64, pdu: &[u8]) -> Vec<u8> {
    let mut sealed = scoped(pdu);
    usm::encrypt(&keys.privacy, usm::iv(BOOTS, TIME, AGENT_SALT), &mut sealed);
    let security = security(setup.user.as_bytes(), &[0; 24], &AGENT_SALT);
    let mut reply = whole(id, 0x03, &security, &ber::octets(&sealed));
    let at = digest_offset(&reply);
    let digest = usm::digest(&keys.auth, &reply);
    reply[at..at + 24].copy_from_slice(&digest);
    reply
}
