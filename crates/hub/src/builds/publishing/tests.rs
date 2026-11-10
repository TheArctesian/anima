//! Tests of the pure publishing rules: `SigV4` against the AWS examples, the feed check, the file
//! set, the updater signature and property tests of the parsers (BLD-060 to BLD-069, D-24,
//! R-108).

mod feeds;
mod files;
mod parsers;
mod signatures;
mod sigv4;

use crate::builds::model::UploadedFile;

fn file(name: &str, bytes: &[u8]) -> UploadedFile {
    UploadedFile {
        name: name.to_owned(),
        sha256: super::sigv4::sha256_hex(bytes),
        size: i64::try_from(bytes.len()).unwrap(),
    }
}
