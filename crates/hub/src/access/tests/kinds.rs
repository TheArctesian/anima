//! Each kind of token travels one way, and an API key is spelled apart (WP-43 A-3, D-23).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest as _, Sha256};

use crate::access::tokens::{
    Spelling, TokenHash, TokenKind, Transport, admitted, hash_of, is_agent_secret, read_presented,
};

#[test]
fn a_3_each_kind_is_admitted_only_from_its_transport_and_spelling() {
    use Spelling::{Key, Plain};
    use TokenKind::{ApiKey, Browser, Login};
    use Transport::{Bearer, Cookie};
    let admitted_cases = [
        (Browser, Plain, Cookie),
        (Login, Plain, Bearer),
        (ApiKey, Key, Bearer),
    ];
    for kind in [Login, Browser, ApiKey] {
        for spelling in [Plain, Key] {
            for transport in [Bearer, Cookie] {
                let case = (kind, spelling, transport);
                assert_eq!(
                    admitted(kind, spelling, transport),
                    admitted_cases.contains(&case),
                    "{case:?}"
                );
            }
        }
    }
}

#[test]
fn d_23_a_key_is_anima_and_a_token_and_hashes_its_32_bytes() {
    let body = URL_SAFE_NO_PAD.encode([4_u8; 32]);
    let expected = TokenHash(Sha256::digest([4_u8; 32]).into());
    let key = read_presented(&format!("anima_{body}"));
    assert!(key == Some((expected, Spelling::Key)));
    assert!(read_presented(&body) == Some((expected, Spelling::Plain)));
    assert!(
        hash_of(&format!("anima_{body}")).is_none(),
        "login rule is unchanged"
    );
    for refused in [
        format!("anima_{}", &body[..42]),
        format!("anima_{body}A"),
        format!("anima_anima_{body}"),
        format!("Anima_{body}"),
        format!("anima-{body}"),
        "anima_".to_owned(),
        String::new(),
    ] {
        assert!(read_presented(&refused).is_none(), "{refused}");
    }
}

#[test]
fn a_3_a_plain_token_that_starts_with_anima_is_still_plain() {
    // 43 characters: base64url of 32 bytes whose text begins with `anima_`.
    let mut bytes = URL_SAFE_NO_PAD
        .decode(format!("anima_{}", "A".repeat(37)))
        .unwrap();
    bytes.truncate(32);
    let text = URL_SAFE_NO_PAD.encode(&bytes);
    assert!(text.starts_with("anima_") && text.len() == 43, "{text}");
    let (_, spelling) = read_presented(&text).expect("a plain token");
    assert_eq!(spelling, Spelling::Plain);
}

#[test]
fn a_3_kind_round_trips_its_stored_text() {
    for kind in [TokenKind::Login, TokenKind::Browser, TokenKind::ApiKey] {
        assert_eq!(TokenKind::from_stored(kind.as_str()), Some(kind));
    }
    assert_eq!(TokenKind::from_stored("API_KEY"), None);
    assert_eq!(TokenKind::from_stored(""), None);
}

#[test]
fn r_092_agent_secrets_are_spelled_by_exact_prefix_length_and_alphabet() {
    let token = URL_SAFE_NO_PAD.encode([9_u8; 32]);
    let code = URL_SAFE_NO_PAD.encode([9_u8; 16]);
    for secret in [
        format!("anima_agent_{token}"),
        format!("anima_enrol_{code}"),
        format!("anima_agent_{}x", "-_aZ09".repeat(7)),
    ] {
        assert!(is_agent_secret(&secret), "{secret}");
        assert!(read_presented(&secret).is_none(), "{secret}");
    }
    for other in [
        // API keys (`anima_` + 43) whose text begins like an agent secret, and a key.
        format!("anima_agent_{}", &token[..37]),
        format!("anima_enrol_{}", &token[..37]),
        format!("anima_{token}"),
        token.clone(),
        // One character short or long, outside the alphabet, or another prefix.
        format!("anima_agent_{}", &token[..42]),
        format!("anima_agent_{token}A"),
        format!("anima_enrol_{}", &code[..21]),
        format!("anima_enrol_{code}A"),
        format!("anima_agent_{}+", &token[..42]),
        format!("anima_enrol_{}=", &code[..21]),
        format!("anima_agent_{}\u{e9}", &token[..41]),
        format!("Anima_agent_{token}"),
        format!("xanima_agent_{}", &token[..42]),
        "anima_agent_".to_owned(),
        String::new(),
    ] {
        assert!(!is_agent_secret(&other), "{other}");
    }
}
