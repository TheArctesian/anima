//! API keys that a provider issues with a fixed prefix (RED-002): Anthropic, `OpenAI`, GitHub,
//! AWS, Google, Slack and Stripe. Such a key is a secret wherever it stands, with no key name
//! or scheme word before it.

use super::{REDACTED, text::starts_word};

/// A key prefix, compared case-sensitively, and the fewest key bytes that must follow it.
const PREFIXES: [(&str, usize); 25] = [
    ("sk-ant-", 20),
    ("sk-proj-", 20),
    ("sk-svcacct-", 20),
    ("sk-", 20),
    ("ghp_", 30),
    ("gho_", 30),
    ("ghu_", 30),
    ("ghs_", 30),
    ("ghr_", 30),
    ("github_pat_", 22),
    ("AKIA", 16),
    ("ASIA", 16),
    ("AIza", 35),
    ("ya29.", 20),
    ("xoxb-", 10),
    ("xoxa-", 10),
    ("xoxp-", 10),
    ("xoxr-", 10),
    ("xoxs-", 10),
    ("xapp-", 10),
    ("sk_live_", 16),
    ("sk_test_", 16),
    ("rk_live_", 16),
    ("rk_test_", 16),
    ("whsec_", 16),
];

/// A byte of a provider key after its prefix.
fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

/// Replaces every provider key: a prefix that starts a word, then enough key bytes with a digit
/// or a capital among them, so `sk-learn-is-a-fine-library` stays. Linear in the length of
/// `text`: a run of key bytes that is not a key is skipped as a whole.
pub(super) fn mask_provider_keys(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let (mut copied, mut index) = (0, 0);
    while index < text.len() {
        match key_len(text, index) {
            Some(len) => {
                masked.push_str(&text[copied..index]);
                masked.push_str(REDACTED);
                index += len;
                copied = index;
            }
            None => index += text[index..].chars().next().map_or(1, char::len_utf8),
        }
    }
    masked.push_str(&text[copied..]);
    masked
}

/// The length of the provider key that starts at byte `index`, prefix included.
fn key_len(text: &str, index: usize) -> Option<usize> {
    if !starts_word(text, index) {
        return None;
    }
    let rest = &text[index..];
    PREFIXES.iter().find_map(|(prefix, fewest)| {
        let body = rest.strip_prefix(prefix)?.as_bytes();
        let len = body.iter().take_while(|byte| is_key_byte(**byte)).count();
        let shaped = body[..len]
            .iter()
            .any(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase());
        (len >= *fewest && shaped).then_some(prefix.len() + len)
    })
}
