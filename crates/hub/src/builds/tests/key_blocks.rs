//! What the hub keeps of a log that prints a private key (BLD-048, BLD-049, LOG-007).

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::builds::log_lines::{self, KEY_BLOCK};

fn lines(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&text| text.to_owned()).collect()
}

/// The lines kept of one report sent outside a key block.
fn stored(report: &[String]) -> Vec<String> {
    log_lines::clean_all(report, None).unwrap().0
}

#[test]
fn bld_049_the_lines_of_a_private_key_block_are_dropped() {
    let report = lines(&[
        "importing the updater key",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW",
        "QyNTUxOQAAACDRmHUsAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "-----END OPENSSH PRIVATE KEY-----",
        "done",
    ]);
    let kept = stored(&report);
    assert_eq!(kept, ["importing the updater key", KEY_BLOCK, "done"]);
    let unended = lines(&["-----BEGIN PRIVATE KEY-----", "MIIEvQIBADANBg", "more"]);
    assert_eq!(stored(&unended), [KEY_BLOCK]);
    let pgp = lines(&[
        "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        "",
        "lQOYBF8AAAABCADnsecretbody",
        "=Xyz1",
        "-----END PGP PRIVATE KEY BLOCK-----",
        "done",
    ]);
    assert_eq!(stored(&pgp), [KEY_BLOCK, "done"]);
    let ssh2 = lines(&[
        "---- BEGIN SSH2 ENCRYPTED PRIVATE KEY ----",
        "Comment: \"rsa-key\"",
        "P2/56wAAA+wAAAA3aWYtbW9kbsecretbody",
        "---- END SSH2 ENCRYPTED PRIVATE KEY ----",
        "done",
    ]);
    assert_eq!(stored(&ssh2), [KEY_BLOCK, "done"]);
    let pgp_secret = lines(&[
        "-----BEGIN PGP SECRET KEY BLOCK-----",
        "Version: GnuPG v2",
        "",
        "lQOYBF8AAAABCADnsecretbody",
        "-----END PGP SECRET KEY BLOCK-----",
        "done",
    ]);
    assert_eq!(stored(&pgp_secret), [KEY_BLOCK, "done"]);
    let one_line = "---- BEGIN SSH2 ENCRYPTED PRIVATE KEY ---- P2/56wAAA+wsecretbody";
    let after = lines(&[one_line, "signed the update"]);
    assert_eq!(stored(&after), [KEY_BLOCK, "signed the update"]);
    assert_eq!(log_lines::clean(one_line), "[redacted]");
    let putty = lines(&[
        "Public-Lines: 1",
        "AAAAC3NzaC1lZDI1NTE5AAAAIPublic",
        "Private-Lines: 1",
        "AAAAIQDsecretbody",
        "Private-MAC: 0f1e2d",
        "done",
    ]);
    assert_eq!(
        stored(&putty),
        [
            "Public-Lines: 1",
            "AAAAC3NzaC1lZDI1NTE5AAAAIPublic",
            KEY_BLOCK,
            "done"
        ]
    );
}

#[test]
fn bld_049_a_key_block_open_at_a_report_end_stays_open_into_the_next() {
    let first = lines(&[
        "key:",
        "-----BEGIN RSA PRIVATE KEY-----",
        "MIIEowIBAAKCAQEA",
    ]);
    let (kept, open) = log_lines::clean_all(&first, None).unwrap();
    assert_eq!(kept, ["key:", KEY_BLOCK]);
    assert_eq!(open, Some(199));
    let second = lines(&["secretbody", "-----END RSA PRIVATE KEY-----", "done"]);
    let (kept, open) = log_lines::clean_all(&second, open).unwrap();
    assert_eq!(kept, ["done"]);
    assert_eq!(open, None);
}

#[test]
fn bld_048_an_opening_line_with_no_key_after_it_keeps_the_rest_of_the_log() {
    let report = lines(&[
        "$ grep -rln -- '-----BEGIN .* PRIVATE KEY' src || true",
        "error[E0308]: mismatched types",
        "Build failed: exit 101",
    ]);
    let (kept, open) = log_lines::clean_all(&report, None).unwrap();
    assert_eq!(
        kept,
        [
            KEY_BLOCK,
            "error[E0308]: mismatched types",
            "Build failed: exit 101"
        ]
    );
    assert_eq!(open, None);
    // A block left open by an earlier report ends at the first line that is not key material.
    let next = lines(&["   Compiling anima-hub v0.1.0", "done"]);
    let (kept, open) = log_lines::clean_all(&next, Some(150)).unwrap();
    assert_eq!(kept, ["   Compiling anima-hub v0.1.0", "done"]);
    assert_eq!(open, None);
}

#[test]
fn bld_048_a_key_block_takes_at_most_200_lines() {
    let mut report = vec!["-----BEGIN RSA PRIVATE KEY-----".to_owned()];
    report.extend((0..201).map(|n| format!("MIIEowIBAAKCAQEA{n:04}")));
    let (kept, open) = log_lines::clean_all(&report, None).unwrap();
    assert_eq!(kept, [KEY_BLOCK, "MIIEowIBAAKCAQEA0200"]);
    assert_eq!(open, None);
}

/// A minisign key file as the Tauri updater writes it, base64 of its two lines.
fn minisign_file(comment: &str, key: &str) -> String {
    STANDARD.encode(format!("untrusted comment: {comment}\n{key}\n"))
}

#[test]
fn bld_049_the_updater_secret_key_is_redacted_and_its_public_key_kept() {
    let secret = "RWRTY0IyhB2rsQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA0secret";
    let report = lines(&[
        "untrusted comment: rsign encrypted secret key",
        secret,
        "signed",
    ]);
    assert_eq!(stored(&report), [KEY_BLOCK, "signed"]);
    let file = minisign_file("rsign encrypted secret key", secret);
    assert!(file.starts_with("dW50cnVzdGVkIGNvbW1lbnQ6"));
    assert_eq!(
        log_lines::clean(&format!("echo {file} > key")),
        "[redacted]"
    );
    let public = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    let shown = lines(&["untrusted comment: minisign public key: 1E1D2C3B4A", public]);
    assert_eq!(stored(&shown), shown);
    let public_file = minisign_file("minisign public key: 1E1D2C3B4A", public);
    let config = format!("\"pubkey\": \"{public_file}\"");
    assert_eq!(log_lines::clean(&config), config);
}

/// BLD-049, LOG-007 (T6 security review C-5): a minisign secret key's base64 line printed on
/// its own, with no comment line before it, is redacted as the client redacts it; a word that
/// only holds those letters inside it is kept.
#[test]
fn bld_049_a_bare_minisign_secret_key_line_is_redacted() {
    for kdf in [&b"Sc"[..], &[0, 0][..]] {
        let mut key = b"Ed".to_vec();
        key.extend_from_slice(kdf);
        key.extend_from_slice(b"B2");
        key.extend_from_slice(&[7; 152]);
        let probe = format!("key {}", STANDARD.encode(&key));
        assert_eq!(log_lines::clean(&probe), "[redacted]", "{probe}");
    }
    let inside = "sha256 0RWRTY0Iy0 ok";
    assert_eq!(log_lines::clean(inside), inside);
}
