//! The enrolment code and the agent token (K8 Section 2, MET-012, MET-013, LOG-007).

use std::fmt::{Debug, Display};

use sha2::{Digest as _, Sha256};
use static_assertions::assert_not_impl_any;

use crate::wire::{
    AgentToken, CODE_BYTES, EnrolRequest, Enrolled, EnrolmentCode, Renewed, TOKEN_BYTES,
};

assert_not_impl_any!(AgentToken: Debug, Display, Clone, serde::Serialize, serde::de::DeserializeOwned);
assert_not_impl_any!(EnrolmentCode: Debug, Display, Clone, serde::Serialize, serde::de::DeserializeOwned);
assert_not_impl_any!(Enrolled: Debug, Display, Clone);
assert_not_impl_any!(Renewed: Debug, Display, Clone);
assert_not_impl_any!(EnrolRequest: Debug, Display, Clone);

const CODE: &str = "anima_enrol_AAECAwQFBgcICQoLDA0ODw";
const TOKEN: &str = "anima_agent_ZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXp7fH1-f4CBgoM";

fn code_bytes() -> [u8; CODE_BYTES] {
    std::array::from_fn(|n| u8::try_from(n).unwrap())
}

fn token_bytes() -> [u8; TOKEN_BYTES] {
    std::array::from_fn(|n| u8::try_from(n + 100).unwrap())
}

#[test]
fn met_012_a_code_is_the_prefix_and_22_base64url_characters_of_its_16_bytes() {
    let code = EnrolmentCode::from_random(code_bytes());
    assert_eq!(code.expose(), CODE);
    let digest: [u8; 32] = Sha256::digest(code_bytes()).into();
    assert_eq!(
        code.hash().to_bytes(),
        digest,
        "the hash is of the bytes, not the text"
    );
    let parsed = EnrolmentCode::parse(CODE).unwrap();
    assert!(parsed.hash() == code.hash());
    assert_eq!(parsed.expose(), CODE);
}

#[test]
fn met_013_a_token_is_the_prefix_and_43_base64url_characters_of_its_32_bytes() {
    let token = AgentToken::from_random(token_bytes());
    assert_eq!(token.expose(), TOKEN);
    let digest: [u8; 32] = Sha256::digest(token_bytes()).into();
    assert_eq!(token.hash().to_bytes(), digest);
    assert!(AgentToken::parse(TOKEN).unwrap().hash() == token.hash());
    let other = AgentToken::from_random([7; TOKEN_BYTES]);
    assert!(other.hash() != token.hash());
}

/// Spellings that are not the canonical text of a token or a code.
fn not_canonical(canonical: &str, prefix: &str) -> Vec<String> {
    let body = &canonical[prefix.len()..];
    let mut last_changed = canonical.to_owned();
    let last = last_changed.pop().unwrap();
    // The next character keeps the bytes' bits but sets a bit past them.
    last_changed.push(char::from(u8::try_from(u32::from(last) + 1).unwrap()));
    vec![
        String::new(),
        body.to_owned(),
        format!("Anima{}", &canonical[5..]),
        format!("anima_{body}"),
        canonical[..canonical.len() - 1].to_owned(),
        format!("{canonical}A"),
        format!("{canonical}="),
        format!("{canonical}=="),
        last_changed,
        format!(" {canonical}"),
        format!("{canonical}\n"),
        canonical.to_uppercase(),
    ]
}

#[test]
fn k8_2_only_the_canonical_spelling_of_a_code_is_accepted() {
    for text in not_canonical(CODE, "anima_enrol_") {
        assert!(EnrolmentCode::parse(&text).is_none(), "{text:?}");
    }
    assert!(
        EnrolmentCode::parse(TOKEN).is_none(),
        "a token is not a code"
    );
    assert!(EnrolmentCode::parse(&CODE.replace("enrol", "agent")).is_none());
}

#[test]
fn k8_2_only_the_canonical_spelling_of_a_token_is_accepted() {
    // The standard alphabet's `+` and `/` in place of base64url's `-` and `_`.
    let with_plus = TOKEN.replace("1-f", "1+f");
    let with_slash = format!("{}/{}", &TOKEN[..20], &TOKEN[21..]);
    for text in not_canonical(TOKEN, "anima_agent_")
        .into_iter()
        .chain([with_plus, with_slash])
    {
        assert!(AgentToken::parse(&text).is_none(), "{text:?}");
    }
    assert!(AgentToken::parse(CODE).is_none(), "a code is not a token");
    let api_key = format!("anima_{}", &TOKEN["anima_agent_".len()..]);
    assert!(
        AgentToken::parse(&api_key).is_none(),
        "an API key is not a token"
    );
}

#[test]
fn k8_2_every_random_value_parses_back_to_the_same_hash() {
    for seed in [0_u8, 1, 0x7f, 0x80, 0xfe, 0xff] {
        let token = AgentToken::from_random([seed; TOKEN_BYTES]);
        assert_eq!(token.expose().len(), "anima_agent_".len() + 43);
        assert!(AgentToken::parse(token.expose()).unwrap().hash() == token.hash());
        let code = EnrolmentCode::from_random([seed; CODE_BYTES]);
        assert_eq!(code.expose().len(), "anima_enrol_".len() + 22);
        assert!(EnrolmentCode::parse(code.expose()).unwrap().hash() == code.hash());
    }
}
