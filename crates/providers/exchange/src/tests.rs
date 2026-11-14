use chrono::{TimeZone as _, Utc};

use super::*;

#[test]
fn src_006_a_provider_request_is_a_get_below_the_api_base() {
    let request = Request::get("/installation/repositories", &[("per_page", "100")]).unwrap();
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, "/installation/repositories");
    assert_eq!(request.query, [("per_page".to_owned(), "100".to_owned())]);
}

#[test]
fn src_006_a_path_that_could_leave_the_api_base_is_refused() {
    for (path, error) in [
        ("repos", RequestError::NotAbsolutePath),
        ("//evil.example/x", RequestError::NotAbsolutePath),
        ("/x?u=https://evil.example", RequestError::NamesAHost),
        ("/x/user@evil.example", RequestError::NamesAHost),
    ] {
        assert_eq!(Request::get(path, &[]), Err(error), "{path}");
    }
}

#[test]
fn src_008_a_path_with_a_dot_segment_or_an_encoded_dot_is_refused() {
    for path in [
        "/repos/o/n/git/trees/../../../../../app/installations",
        "/repos/./x",
        "/repos/x/..",
        "/repos/%2E%2E/app",
        "/repos/%2e/x",
        "/repos/a%2Eb",
    ] {
        assert_eq!(
            Request::get(path, &[]),
            Err(RequestError::DotSegment),
            "{path}"
        );
    }
    assert!(Request::get("/repos/o/n.js/git/trees/v1.2", &[]).is_ok());
}

fn mint_path(installation: &str) -> String {
    format!("/app/installations/{installation}/access_tokens")
}

fn parse_minted(body: &[u8]) -> Result<Minted, Problem> {
    if body.is_empty() {
        return Err(Problem {
            code: "empty_reply",
            subject: "token".to_owned(),
            message: "the reply was empty".to_owned(),
        });
    }
    Ok(Minted {
        token: "t".to_owned(),
        expires_at: Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
    })
}

#[test]
fn src_021_the_poll_interval_has_a_floor_of_fifteen_minutes() {
    let declaration = |poll_interval_seconds| Declaration {
        key: "test",
        name: "Test",
        acquisition: Acquisition::TokenPull(Minting::AppKey(AppKey {
            mint_path,
            parse_minted,
        })),
        scopes: &[],
        write_reason: None,
        gives: &[],
        poll_interval_seconds,
        api_base: "https://api.example",
    };
    assert!(declaration(900).poll_interval_is_allowed());
    assert!(!declaration(899).poll_interval_is_allowed());
}

#[test]
fn src_001_a_declaration_names_its_mint_path_for_an_installation() {
    let key = AppKey {
        mint_path,
        parse_minted,
    };
    assert_eq!((key.mint_path)("42"), "/app/installations/42/access_tokens");
    assert!((key.parse_minted)(b"{}").is_ok());
    assert_eq!((key.parse_minted)(b"").err().unwrap().code, "empty_reply");
}
