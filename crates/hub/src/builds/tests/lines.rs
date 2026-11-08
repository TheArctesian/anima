//! What the hub keeps of a log line (BLD-048, BLD-049, LOG-007).

use proptest::prelude::*;

use crate::builds::log_lines::{self, MAX_LINES_PER_REPORT};

const TOKEN: &str = "dGhpcyBpcyBhIGJ1aWxkZXIgdG9rZW4gb2YgMzIgYnl0";

fn lines(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&text| text.to_owned()).collect()
}

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
    let kept = log_lines::clean_all(&report).unwrap();
    assert_eq!(kept, ["importing the updater key", "[redacted]", "done"]);
    let unended = lines(&["-----BEGIN PRIVATE KEY-----", "MIIEvQIBADANBg", "more"]);
    assert_eq!(log_lines::clean_all(&unended).unwrap(), ["[redacted]"]);
}

#[test]
fn bld_048_blank_lines_are_not_kept() {
    let report = lines(&["", "  ", "\t", "\x1b[0m", "\u{200b}", "kept"]);
    assert_eq!(log_lines::clean_all(&report).unwrap(), ["kept"]);
}

#[test]
fn bld_048_control_characters_become_spaces_and_tabs_stay() {
    assert_eq!(log_lines::clean("a\0b\x1b[31mc\r\nd\te"), "a bc  d\te");
}

#[test]
fn bld_048_a_report_carries_at_most_1000_lines() {
    let many = vec!["x".to_owned(); MAX_LINES_PER_REPORT + 1];
    assert_eq!(log_lines::clean_all(&many).unwrap_err().field, "logLines");
    let enough = vec!["x".to_owned(); MAX_LINES_PER_REPORT];
    assert_eq!(
        log_lines::clean_all(&enough).unwrap().len(),
        MAX_LINES_PER_REPORT
    );
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
        prop_assert!(kept.chars().all(|c| !crate::builds::hidden_text::is_format(c)));
    }
}
