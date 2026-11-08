//! What the hub keeps of a log line (BLD-048, BLD-049, LOG-007).

use base64::{Engine, engine::general_purpose::STANDARD};
use proptest::prelude::*;

use crate::builds::log_lines::{self, KEY_BLOCK, MAX_LINES_PER_REPORT};

const TOKEN: &str = "dGhpcyBpcyBhIGJ1aWxkZXIgdG9rZW4gb2YgMzIgYnl0";

fn lines(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&text| text.to_owned()).collect()
}

/// The lines kept of one report sent outside a key block.
fn stored(report: &[String]) -> Vec<String> {
    log_lines::clean_all(report, None).unwrap().0
}

/// Keys that name a secret, in the spellings builds print.
const SECRET_KEYS: [&str; 10] = [
    "password",
    "secret",
    "token",
    "passwd",
    "api_key",
    "PGPASSWORD",
    "GITHUB_TOKEN",
    "client_secret",
    "pass",
    "X-Amz-Signature",
];

/// Pieces that split a key: escapes that take the letter after them (`ESC [`, `ESC (`, C1
/// CSI), and whole sequences and hidden characters that take nothing.
const SPLIT_PIECES: [&str; 13] = [
    "\x1b[",
    "\x1b[1;",
    "\x1b(",
    "\u{9b}",
    "\x1b",
    "\x1b[0m",
    "\x1b[m",
    "\x1b[31m",
    "\x1b(B",
    "\x1b7",
    "\u{200b}",
    "\u{fe0f}",
    "\x1b]0;t\x07",
];

/// Escape-sequence pieces a hostile builder may put before a key: whole sequences, sequences
/// split across the key (`ESC [` then a letter final), bare `ESC`s, C1 introducers and hidden
/// characters.
const ESCAPE_PIECES: [&str; 20] = [
    "\x1b",
    "\x1b[",
    "\x1b[1;",
    "\x1b[31m",
    "\x1b[?25",
    "\x1b(",
    "\x1b)",
    "\x1b#",
    "\x1b ",
    "\x1b]0;t\x07",
    "\x1b]",
    "\x1bP",
    "\x1b_",
    "\u{9b}",
    "\u{9b}1;",
    "\u{9d}",
    "\u{200b}",
    "\u{fe0f}",
    "\u{202e}",
    " ",
];

#[test]
fn bld_049_a_line_holding_a_credential_is_redacted_before_it_is_kept() {
    let lines = [
        format!("curl -H 'Authorization: Bearer {TOKEN}' https://hub"),
        format!("GITHUB_TOKEN={TOKEN} bun run gates"),
        format!("upload to https://user:{TOKEN}@releases.anima.ing/x"),
        format!("token\0: {TOKEN}"),
    ];
    for line in &lines {
        let kept = log_lines::clean(line);
        assert!(!kept.contains(TOKEN), "{line:?} kept as {kept:?}");
    }
    assert_eq!(
        log_lines::clean("Compiling anima-hub v0.1.0"),
        "Compiling anima-hub v0.1.0"
    );
}

#[test]
fn bld_049_a_credential_dressed_in_terminal_escapes_is_still_redacted() {
    let lines = [
        format!("\x1b[32mGITHUB_TOKEN\x1b[0m={TOKEN}"),
        format!("\x1b[1;32mGITHUB_TOKEN\x1b[0m: {TOKEN}"),
        format!("GITHUB_\x1b(BTOKEN={TOKEN}"),
        format!("\u{9b}31mpassword\u{9b}0m={TOKEN}"),
        format!("\x1b]0;title\x07api_key={TOKEN}"),
        format!("\x1b]8;;https://x/\x1b\\secret\x1b]8;;\x1b\\={TOKEN}"),
        format!("GITHUB\u{200b}_TOKEN={TOKEN}"),
        format!("\u{202e}GITHUB_TOKEN={TOKEN}"),
    ];
    for line in &lines {
        let kept = log_lines::clean(line);
        assert!(!kept.contains(TOKEN), "{line:?} kept as {kept:?}");
    }
}

#[test]
fn bld_049_a_terminal_escape_is_removed_whole() {
    assert_eq!(
        log_lines::clean("\x1b[1;31merror\x1b[0m: \x1b]0;t\x07done\x1b[?25l"),
        "error: done"
    );
    assert_eq!(log_lines::clean("cut \x1b]0;never ended"), "cut ");
}

#[test]
fn bld_049_a_presigned_url_signature_is_redacted() {
    let signature = "4c1d2e9f".repeat(8);
    let url = format!(
        "PUT https://r2.example/b/x?X-Amz-Credential=AKIA%2F20301&X-Amz-Signature={signature}&X-Amz-Expires=1800"
    );
    let kept = log_lines::clean(&url);
    assert!(!kept.contains(&signature), "{kept}");
    assert!(kept.contains("X-Amz-Expires=1800"), "{kept}");
    let header = format!("x-amz-signature: \"{signature}\"");
    assert!(!log_lines::clean(&header).contains(&signature));
    assert_eq!(
        log_lines::clean("signatures=2 verified"),
        "signatures=2 verified"
    );
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

#[test]
fn bld_049_two_splits_in_one_key_do_not_unmask_its_value() {
    for line in [
        "\x1b[pass\x1b[0mword=hunter2hunter2",
        "\x1b[pass\u{200b}word=hunter2hunter2",
        "pa\x1b[ss\x1b[word=hunter2hunter2",
    ] {
        let kept = log_lines::clean(line);
        assert!(
            !kept.contains("hunter2hunter2"),
            "{line:?} kept as {kept:?}"
        );
    }
}

#[test]
fn bld_049_a_hidden_character_does_not_cost_the_rest_of_the_line() {
    assert_eq!(
        log_lines::clean("\u{2714}\u{fe0f} uploaded token_count=3 files"),
        "\u{2714} uploaded token_count=[redacted] files"
    );
}

#[test]
fn bld_049_an_escape_that_hides_a_key_letter_does_not_unmask_its_value() {
    let lines = [
        format!("\x1bpassword={TOKEN}"),
        format!("\x1bsecret={TOKEN}"),
        format!("\x1b#pass={TOKEN}"),
        format!("\x1b[password={TOKEN}"),
        format!("\x1bPassword={TOKEN}"),
        format!("\x1b(password={TOKEN}"),
        format!("pass\u{fe0f}word={TOKEN}"),
        format!("\x1btoken: {TOKEN}"),
        format!("\x1b[?25pass={TOKEN}"),
        format!("\x1b]\x1b[31mpass={TOKEN}"),
    ];
    for line in &lines {
        let kept = log_lines::clean(line);
        assert!(!kept.contains(TOKEN), "{line:?} kept as {kept:?}");
    }
    // A bare ESC before a letter takes only itself; the escapes terminals use go whole.
    assert_eq!(log_lines::clean("\x1bpwd ok"), "pwd ok");
    assert_eq!(
        log_lines::clean("\x1b7saved\x1b8 \x1b(Bdone\x1bM"),
        "saved done"
    );
}

#[test]
fn bld_049_a_percent_encoded_signature_is_redacted() {
    let signature = "4c1d2e9f".repeat(8);
    for url in [
        format!("next=%2Fput%3FX-Amz-Signature%3D{signature}%26X-Amz-Expires%3D1800"),
        format!("x-amz-signature%3a{signature}"),
    ] {
        let kept = log_lines::clean(&url);
        assert!(!kept.contains(&signature), "{kept}");
    }
    assert!(
        log_lines::clean(&format!(
            "X-Amz-Signature%3d{signature}%26X-Amz-Expires%3D1800"
        ))
        .ends_with("%26X-Amz-Expires%3D1800")
    );
}

#[test]
fn bld_048_blank_lines_are_not_kept() {
    let report = lines(&["", "  ", "\t", "\x1b[0m", "\u{200b}", "kept"]);
    assert_eq!(stored(&report), ["kept"]);
}

#[test]
fn bld_048_control_characters_become_spaces_and_tabs_stay() {
    assert_eq!(log_lines::clean("a\0b\x1b[31mc\r\nd\te"), "a bc  d\te");
}

#[test]
fn bld_048_a_report_carries_at_most_1000_lines() {
    let many = vec!["x".to_owned(); MAX_LINES_PER_REPORT + 1];
    assert_eq!(
        log_lines::clean_all(&many, None).unwrap_err().field,
        "logLines"
    );
    let enough = vec!["x".to_owned(); MAX_LINES_PER_REPORT];
    assert_eq!(stored(&enough).len(), MAX_LINES_PER_REPORT);
}

#[test]
fn bld_010_a_failure_reason_is_cleaned_cut_and_never_empty() {
    assert_eq!(log_lines::reason(Some("  gates failed\n")), "gates failed");
    assert_eq!(log_lines::reason(None), "the builder reported a failure");
    assert_eq!(
        log_lines::reason(Some(&"é".repeat(300))).chars().count(),
        200
    );
    let secret = log_lines::reason(Some(&format!("token: {TOKEN}")));
    assert!(!secret.contains(TOKEN), "{secret}");
}

proptest! {
    #[test]
    fn bld_048_a_kept_line_is_at_most_4096_bytes_with_no_control_but_tab(line in "\\PC*|.{0,6000}") {
        let kept = log_lines::clean(&line);
        prop_assert!(kept.len() <= 4096);
        prop_assert!(kept.chars().all(|c| !c.is_control() || c == '\t'));
    }

    #[test]
    fn bld_049_no_escape_or_format_character_survives(line in "[\\x00-\\x7f\\u{80}-\\u{9f}\\u{200b}-\\u{202e}a-z=]{0,64}") {
        let kept = log_lines::clean(&line);
        prop_assert!(!kept.contains(char::from(0x1b_u8)));
        prop_assert!(kept.chars().all(|c| !crate::builds::hidden_text::is_hidden(c)));
    }

    #[test]
    fn bld_049_no_two_splits_in_a_key_unmask_its_value(
        key in proptest::sample::select(&SECRET_KEYS[..]),
        cuts in (0..=20_usize, 0..=20_usize),
        first in proptest::sample::select(&SPLIT_PIECES[..]),
        second in proptest::sample::select(&SPLIT_PIECES[..]),
        value in "Q[0-9]{8}[a-z0-9]{0,8}",
    ) {
        // Split points inside the key: before its first letter up to before its last one. An
        // escape between the key and `=` can take the `=` itself (`ESC =`), and then neither a
        // terminal nor redaction reads a separator.
        let (a, b) = (cuts.0 % key.len(), cuts.1 % key.len());
        let (a, b) = (a.min(b), a.max(b));
        let line = format!("{}{first}{}{second}{}={value}", &key[..a], &key[a..b], &key[b..]);
        let kept = log_lines::clean(&line);
        prop_assert!(!kept.contains(&value), "{:?} kept as {:?}", line, kept);
    }

    #[test]
    fn bld_049_no_escape_prefix_unmasks_a_secret_value(
        pieces in proptest::collection::vec(proptest::sample::select(&ESCAPE_PIECES[..]), 1..5),
        key in proptest::sample::select(&SECRET_KEYS[..]),
        separator in proptest::sample::select(&["=", ": ", "=\""][..]),
        value in "Q[0-9]{8}[a-z0-9]{0,8}",
    ) {
        let line = format!("{}{key}{separator}{value}", pieces.concat());
        let kept = log_lines::clean(&line);
        prop_assert!(!kept.contains(&value), "{:?} kept as {:?}", line, kept);
    }
}
