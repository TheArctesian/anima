//! The `.anrec` format (RA-040, D-13, K4 §11): round trips, the STREAM nonce layout, and
//! every change to a file read as damage, a clean crash read as incomplete.

mod layout;
mod properties;
mod round_trip;
mod tamper;

use std::io;

use uuid::Uuid;

use super::{
    Format, HEADER_LEN, Header, NONCE_PREFIX_LEN, ReadError, Reader, RecordingKey, Sealer, Step,
};
use crate::ids::RecordingId;

const KEY: [u8; 32] = [7; 32];
const PREFIX: [u8; NONCE_PREFIX_LEN] = [9; NONCE_PREFIX_LEN];

fn recording() -> RecordingId {
    RecordingId::from(Uuid::from_u128(0x0123_4567_89ab_cdef_0011_2233_4455_6677))
}

fn key() -> RecordingKey {
    RecordingKey::from_bytes(KEY)
}

fn header() -> Header {
    Header::new(Format::Guacamole, recording(), PREFIX)
}

/// A sealed file split into its header and each framed part, the last part last.
struct Sealed {
    header: [u8; HEADER_LEN],
    parts: Vec<Vec<u8>>,
}

impl Sealed {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_vec();
        for part in &self.parts {
            bytes.extend_from_slice(part);
        }
        bytes
    }
}

/// Seals `parts` before the last, then `last` with the last flag.
fn seal(parts: &[&[u8]], last: &[u8]) -> Sealed {
    let mut sealer = Sealer::new(key(), &header());
    let header = *sealer.header();
    let mut framed: Vec<Vec<u8>> = parts
        .iter()
        .map(|part| sealer.seal(part).expect("a test part is 1 to 64 KiB"))
        .collect();
    framed.push(sealer.finish(last).expect("a test part is at most 64 KiB"));
    Sealed {
        header,
        parts: framed,
    }
}

/// How a read of a whole file ended.
#[derive(Debug, PartialEq, Eq)]
enum End {
    Done,
    Incomplete,
    Failed(ReadError),
}

/// Reads every part of `source` with the test key, expecting the test recording.
fn read_from(source: impl io::Read) -> (Vec<Vec<u8>>, End) {
    let mut reader = match Reader::open(source, key(), recording()) {
        Ok(reader) => reader,
        Err(error) => return (Vec::new(), End::Failed(error)),
    };
    let mut parts = Vec::new();
    loop {
        match reader.next_part() {
            Ok(Step::Part(plaintext)) => parts.push(plaintext.to_vec()),
            Ok(Step::Done) => return (parts, End::Done),
            Ok(Step::Incomplete) => return (parts, End::Incomplete),
            Err(error) => return (parts, End::Failed(error)),
        }
    }
}

fn read(bytes: &[u8]) -> (Vec<Vec<u8>>, End) {
    read_from(bytes)
}

fn damaged(damage: super::Damage) -> End {
    End::Failed(ReadError::Damaged(damage))
}
