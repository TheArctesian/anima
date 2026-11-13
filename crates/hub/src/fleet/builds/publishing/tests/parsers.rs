//! Property tests of the parsers that read builder and bucket bytes (BLD-064, BLD-065, UPD-010):
//! none panics on arbitrary input, and a file read back passes only with exactly its reported
//! size and digest, however its bytes are split into chunks.

use proptest::prelude::*;
use serde_json::json;

use super::file;
use crate::fleet::builds::{
    model::{Target, UploadedFile},
    publishing::{
        feed::{Expected, check},
        files::{Received, check_sums},
        updater_key::UpdaterKey,
    },
};

fn files() -> Vec<UploadedFile> {
    vec![
        file("Anima_0.1.12_amd64.AppImage", b"app"),
        file("SHA256SUMS", b"sums"),
    ]
}

fn expected(files: &[UploadedFile]) -> Expected<'_> {
    Expected {
        target: Target::LinuxX86_64,
        version: "0.1.12".parse().unwrap(),
        public_base: "https://releases.anima.ing",
        files,
        floor: None,
    }
}

/// Reads `bytes` as the reply for `reported`, in chunks cut at `cuts`.
fn read_back(reported: &UploadedFile, bytes: &[u8], cuts: &[usize]) -> Result<Vec<u8>, String> {
    let mut received = Received::new(reported);
    let mut start = 0;
    let mut ends: Vec<usize> = cuts.iter().map(|cut| cut % (bytes.len() + 1)).collect();
    ends.push(bytes.len());
    ends.sort_unstable();
    for end in ends {
        received.push(&bytes[start..end])?;
        start = end;
    }
    received.finish()
}

proptest! {
    #[test]
    fn bld_065_any_feed_text_is_checked_without_a_panic(text in ".{0,512}") {
        let files = files();
        let _ = check(&text, &expected(&files));
    }

    #[test]
    fn bld_065_any_json_shape_is_checked_without_a_panic(
        version in "[0-9.]{0,8}",
        key in "[a-z0-9-]{0,16}",
        url in ".{0,64}",
        signature in ".{0,64}",
    ) {
        let files = files();
        let feed = json!({
            "version": version,
            "platforms": { key: { "signature": signature, "url": url } },
        });
        prop_assert!(check(&feed.to_string(), &expected(&files)).is_err());
    }

    #[test]
    fn upd_010_any_sums_bytes_are_checked_without_a_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let _ = check_sums(&bytes, &files());
    }

    #[test]
    fn r_117_any_key_password_or_public_key_text_is_read_without_a_panic(
        key in prop::collection::vec(any::<u8>(), 0..256),
        password in prop::collection::vec(any::<u8>(), 0..64),
        public in ".{0,256}",
    ) {
        prop_assert!(UpdaterKey::from_parts(&key, &password, &public).is_err());
    }

    #[test]
    fn bld_064_a_read_back_passes_only_with_its_own_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..256),
        other in prop::collection::vec(any::<u8>(), 0..256),
        cuts in prop::collection::vec(any::<usize>(), 0..6),
    ) {
        let reported = file("SHA256SUMS", &bytes);
        prop_assert_eq!(read_back(&reported, &bytes, &cuts), Ok(bytes.clone()));
        prop_assert_eq!(read_back(&reported, &other, &cuts).is_ok(), other == bytes);
    }
}
