//! What the hub keeps of a log line (BLD-048, BLD-049, LOG-007).

use proptest::prelude::*;

use crate::builds::log_lines::{self, MAX_LINES_PER_REPORT};

const TOKEN: &str = "dGhpcyBpcyBhIGJ1aWxkZXIgdG9rZW4gb2YgMzIgYnl0";

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
    assert_eq!(log_lines::clean("Compiling anima-hub v0.1.0"), "Compiling anima-hub v0.1.0");
}

#[test]
fn bld_048_control_characters_become_spaces_and_tabs_stay() {
    assert_eq!(log_lines::clean("a\0b\x1b[31mc\r\nd\te"), "a b [31mc  d\te");
}

#[test]
fn bld_048_a_report_carries_at_most_1000_lines() {
    let many = vec!["x".to_owned(); MAX_LINES_PER_REPORT + 1];
    assert_eq!(log_lines::clean_all(&many).unwrap_err().field, "logLines");
    let enough = vec!["x".to_owned(); MAX_LINES_PER_REPORT];
    assert_eq!(log_lines::clean_all(&enough).unwrap().len(), MAX_LINES_PER_REPORT);
}

#[test]
fn bld_010_a_failure_reason_is_cleaned_cut_and_never_empty() {
    assert_eq!(log_lines::reason(Some("  gates failed\n")), "gates failed");
    assert_eq!(log_lines::reason(None), "the builder reported a failure");
    assert_eq!(log_lines::reason(Some(&"é".repeat(300))).chars().count(), 200);
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
}
