//! K8 Sections 2 and 5: the agent token is read from one `Authorization: Bearer` header, and
//! only its canonical spelling; anything else is no credential.

use agent_core::wire::{AgentToken, EnrolmentCode};
use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

use crate::metrics::ingest::credential::bearer;

fn token() -> AgentToken {
    AgentToken::from_random([7; 32])
}

fn headers(values: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
    }
    headers
}

fn read(values: &[&str]) -> Option<[u8; 32]> {
    bearer(&headers(values)).map(|token| token.hash().to_bytes())
}

#[test]
fn met_013_one_bearer_header_with_the_token_is_read_in_any_scheme_case() {
    let token = token();
    let text = token.expose();
    let expected = Some(token.hash().to_bytes());
    assert_eq!(read(&[&format!("Bearer {text}")]), expected);
    assert_eq!(read(&[&format!("bearer {text}")]), expected);
    assert_eq!(read(&[&format!("BEARER  {text} ")]), expected);
}

#[test]
fn met_013_anything_but_one_bearer_agent_token_is_no_credential() {
    let token = token();
    let text = token.expose();
    let code = EnrolmentCode::from_random([7; 16]);
    let api_key = format!("anima_{}", &text["anima_agent_".len()..]);
    for values in [
        vec![],
        vec![text.to_owned()],
        vec![format!("Basic {text}")],
        vec![format!("Bearer{text}")],
        vec![format!("Bearer {text}"), format!("Bearer {text}")],
        vec![format!("Bearer {}", code.expose())],
        vec![format!("Bearer {api_key}")],
        vec![format!("Bearer {text}=")],
        vec![format!("Bearer {}", &text[..text.len() - 1])],
        vec!["Bearer ".to_owned()],
    ] {
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        assert_eq!(read(&values), None, "{}", values.len());
    }
}
