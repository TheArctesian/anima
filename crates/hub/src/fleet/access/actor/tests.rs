//! The actor kind decision and the agent declaration (LOG-014 to LOG-017, LOG-019).

use axum::http::{HeaderMap, HeaderValue};
use serde_json::json;

use super::{Declaration, MadeFor, Signals, actor, actor_kind};
use crate::fleet::{
    access::tokens::TokenKind,
    events::{self, ActorKind},
};

const TOKENS: [TokenKind; 4] = [
    TokenKind::Login,
    TokenKind::Browser,
    TokenKind::ApiKey,
    TokenKind::Builder,
];
const MADE_FOR: [Option<MadeFor>; 3] = [None, Some(MadeFor::User), Some(MadeFor::Agent)];
const DECLARED: [Declaration; 2] = [Declaration(false), Declaration::AGENT];

/// The expected kind for one combination, written as the table of LOG-014 and LOG-019.
fn expected(token: TokenKind, made_for: Option<MadeFor>, declared: bool) -> ActorKind {
    match (token, made_for, declared) {
        (TokenKind::Builder, _, _) => ActorKind::Hub,
        (_, Some(MadeFor::Agent), _) | (_, _, true) => ActorKind::Agent,
        _ => ActorKind::User,
    }
}

#[test]
fn no_request_is_hub() {
    assert_eq!(actor_kind(None), ActorKind::Hub);
}

#[test]
fn every_combination_follows_the_rules() {
    for token in TOKENS {
        for made_for in MADE_FOR {
            for declared in DECLARED {
                let signals = Signals {
                    token,
                    made_for,
                    declared,
                };
                assert_eq!(
                    actor_kind(Some(signals)),
                    expected(token, made_for, declared.0),
                    "{signals:?}"
                );
            }
        }
    }
}

#[test]
fn nothing_lowers_an_agent_key_or_a_declaration() {
    for token in [TokenKind::Login, TokenKind::Browser, TokenKind::ApiKey] {
        for made_for in MADE_FOR {
            for declared in DECLARED {
                let kind = actor_kind(Some(Signals {
                    token,
                    made_for,
                    declared,
                }));
                if made_for == Some(MadeFor::Agent) || declared.0 {
                    assert_eq!(kind, ActorKind::Agent);
                }
                assert_ne!(kind, ActorKind::Hub, "only no request or a builder is Hub");
            }
        }
    }
}

#[test]
fn the_recorded_actor_names_only_an_api_key() {
    let key = Signals {
        token: TokenKind::ApiKey,
        made_for: Some(MadeFor::Agent),
        declared: Declaration::default(),
    };
    let recorded = actor(key, Some("deploy".to_owned()));
    assert_eq!(recorded.kind(), ActorKind::Agent);
    assert_eq!(recorded.token(), Some(events::TokenKind::ApiKey));
    assert_eq!(recorded.api_key_name(), Some("deploy"));

    let login = Signals {
        token: TokenKind::Login,
        made_for: None,
        declared: Declaration::AGENT,
    };
    let recorded = actor(login, Some("not a key".to_owned()));
    assert_eq!(recorded.kind(), ActorKind::Agent);
    assert_eq!(recorded.token(), Some(events::TokenKind::Login));
    assert_eq!(recorded.api_key_name(), None);

    let builder = Signals {
        token: TokenKind::Builder,
        made_for: None,
        declared: Declaration::AGENT,
    };
    assert_eq!(actor(builder, None), events::Actor::hub());
}

fn headers(values: &[&[u8]]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append("Anima-Actor", HeaderValue::from_bytes(value).unwrap());
    }
    headers
}

#[test]
fn only_the_word_agent_declares_in_a_header() {
    for value in [&b"agent"[..], b"AGENT", b"Agent", b" agent "] {
        assert_eq!(
            Declaration::of_headers(&headers(&[value])),
            Declaration::AGENT
        );
    }
    for value in [
        &b"user"[..],
        b"hub",
        b"",
        b"agents",
        b"agent, agent",
        b"ag\xffent",
        b"\xe4gent",
    ] {
        assert_eq!(
            Declaration::of_headers(&headers(&[value])),
            Declaration::default(),
            "{value:?}"
        );
    }
    assert_eq!(
        Declaration::of_headers(&HeaderMap::new()),
        Declaration::default()
    );
    assert_eq!(
        Declaration::of_headers(&headers(&[b"agent", b"agent"])),
        Declaration::default(),
        "two headers declare nothing"
    );
}

#[test]
fn only_the_word_agent_declares_in_a_payload() {
    assert_eq!(
        Declaration::of_payload(&json!({ "actor": "Agent" })),
        Declaration::AGENT
    );
    for payload in [
        json!({}),
        json!(null),
        json!("agent"),
        json!({ "actor": "user" }),
        json!({ "actor": "hub" }),
        json!({ "actor": true }),
        json!({ "actor": ["agent"] }),
        json!({ "actor": { "kind": "agent" } }),
        json!({ "Actor": "agent" }),
    ] {
        assert_eq!(
            Declaration::of_payload(&payload),
            Declaration::default(),
            "{payload}"
        );
    }
}

/// Any byte string as a header value never panics and declares only for the word itself.
#[test]
fn arbitrary_header_bytes_never_panic() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..5_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let length = usize::try_from(state >> 60).unwrap();
        let bytes: Vec<u8> = (0..length)
            .map(|index| {
                state
                    .rotate_left(u32::try_from(index * 8).unwrap())
                    .to_le_bytes()[0]
            })
            .collect();
        let Ok(value) = HeaderValue::from_bytes(&bytes) else {
            continue;
        };
        let declared = Declaration::of_headers(&headers(&[value.as_bytes()]));
        let is_word =
            std::str::from_utf8(&bytes).is_ok_and(|text| text.trim().eq_ignore_ascii_case("agent"));
        assert_eq!(declared.0, is_word, "{bytes:?}");
    }
}
