//! Secrets stay out of Debug output, stored JSON and failure messages (LOG-007).

use base64::Engine as _;

use super::support::{Reply, env, serve, target};
use crate::checks::{
    config::{
        CheckConfig, HttpAuth, HttpCredentials, HttpHeader, HttpInput, HttpMethod, HttpSecrets,
        Reason, StatusRange,
    },
    probe::probe,
};
use crate::credentials::Secret;

const PASSWORD: &str = "hunter2-PASSWORD";
const TOKEN: &str = "tok-BEARER-123";
const COOKIE: &str = "sid=COOKIE-VALUE";

fn input(url: String, credentials: HttpCredentials) -> HttpInput {
    HttpInput {
        url,
        method: HttpMethod::Get,
        headers: vec![
            ("X-Plain".into(), Secret::from("plain-value".to_owned())),
            ("cookie".into(), Secret::from(COOKIE.to_owned())),
            (
                "Proxy-Authorization".into(),
                Secret::from("Basic PROXY-SECRET".to_owned()),
            ),
        ],
        body: None,
        max_redirects: 10,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials,
        verify_tls: true,
        timeout_ms: 3000,
    }
}

fn basic() -> HttpCredentials {
    HttpCredentials::Basic {
        username: "alice".into(),
        password: Secret::from(PASSWORD.to_owned()),
    }
}

const SECRETS: [&str; 4] = [PASSWORD, "PROXY-SECRET", COOKIE, "COOKIE-VALUE"];

#[test]
fn split_keeps_secrets_out_of_debug_and_the_stored_json() {
    let (config, secrets) = input("http://a.test/".into(), basic()).split().unwrap();
    let debug = format!("{config:?}");
    let json = serde_json::to_string(&CheckConfig::Http(config.clone())).unwrap();
    for text in [
        debug,
        json,
        format!("{:?}", CheckConfig::Http(config.clone())),
    ] {
        for secret in SECRETS {
            assert!(!text.contains(secret), "{secret} leaked into {text}");
        }
        assert!(
            text.contains("plain-value"),
            "non-secret header stays visible"
        );
    }
    assert_eq!(
        config.auth,
        HttpAuth::Basic {
            username: "alice".into()
        }
    );
    assert_eq!(config.headers[1].value, None);
    assert_eq!(config.headers[2].value, None);
    assert_eq!(secrets.headers.len(), 2);
    assert!(secrets.password.is_some());
}

#[test]
fn bearer_token_is_split_out_too() {
    let credentials = HttpCredentials::Bearer(Secret::from(TOKEN.to_owned()));
    let (config, secrets) = input("http://a.test/".into(), credentials).split().unwrap();
    assert_eq!(config.auth, HttpAuth::Bearer);
    assert!(!serde_json::to_string(&config).unwrap().contains(TOKEN));
    assert_eq!(
        secrets.bearer_token.as_ref().unwrap().expose(),
        TOKEN.as_bytes()
    );
}

#[test]
fn secrets_round_trip_through_their_sealed_layout_and_join_back() {
    let (config, secrets) = input("http://a.test/".into(), basic()).split().unwrap();
    let decoded = HttpSecrets::decode(&secrets.encode()).unwrap();
    assert_eq!(
        decoded.password.as_ref().unwrap().expose(),
        PASSWORD.as_bytes()
    );
    assert_eq!(decoded.headers[0].expose(), COOKIE.as_bytes());
    assert_eq!(decoded.headers[1].expose(), b"Basic PROXY-SECRET");
    let joined = HttpInput::join(config.clone(), decoded).unwrap();
    let (again, again_secrets) = joined.split().unwrap();
    assert_eq!(again, config);
    assert_eq!(again_secrets.headers.len(), 2);
    assert!(HttpSecrets::decode(&Secret::from("{".to_owned())).is_err());
}

#[test]
fn sec_005_a_header_that_names_the_destination_is_refused_without_its_value() {
    for name in [
        "Host",
        "host",
        "Forwarded",
        "X-Forwarded-Host",
        "x-forwarded-server",
        "X-Host",
    ] {
        let mut with = input("http://a.test/".into(), basic());
        with.headers
            .push((name.into(), Secret::from("evil.example".to_owned())));
        let error = with.split().err().unwrap();
        assert_eq!(
            (error.field, error.reason),
            ("headers", Reason::Destination),
            "{name}"
        );
        assert!(!error.to_string().contains("evil"), "{name}");
        let (mut stored, _) = input("http://a.test/".into(), basic()).split().unwrap();
        stored.headers.push(HttpHeader {
            name: name.into(),
            value: Some("evil.example".into()),
        });
        assert_eq!(
            stored.validate().unwrap_err().reason,
            Reason::Destination,
            "{name}"
        );
    }
    let mut pseudo = input("http://a.test/".into(), basic());
    pseudo
        .headers
        .push((":authority".into(), Secret::from("evil.example".to_owned())));
    assert_eq!(pseudo.split().err().unwrap().reason, Reason::Invalid);
}

#[test]
fn join_without_a_needed_secret_names_the_field_only() {
    let (config, _) = input("http://a.test/".into(), basic()).split().unwrap();
    let error = HttpInput::join(config, HttpSecrets::default())
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "headers: needs a value that is missing");
}

#[test]
fn split_errors_never_echo_a_value() {
    let mut bad = input("http://a.test/".into(), basic());
    bad.headers.push((
        "Authorization".into(),
        Secret::from("Bearer SECRET\r\nX: y".to_owned()),
    ));
    let error = bad.split().err().unwrap();
    assert_eq!(error.field, "headers");
    assert!(!error.to_string().contains("SECRET"));
    let userinfo = input("http://user:URLPASS@a.test/".into(), basic())
        .split()
        .err()
        .unwrap();
    assert!(!userinfo.to_string().contains("URLPASS"));
}

#[tokio::test]
async fn basic_auth_and_secret_headers_reach_the_server() {
    let server = serve(None, |_| Reply::ok("")).await;
    let (config, secrets) = input(server.url("/"), basic()).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env(),
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    let request = &server.seen()[0];
    let expected = base64::engine::general_purpose::STANDARD.encode(format!("alice:{PASSWORD}"));
    assert_eq!(
        request.header("authorization"),
        Some(format!("Basic {expected}").as_str())
    );
    assert_eq!(request.header("cookie"), Some(COOKIE));
    assert_eq!(
        request.header("proxy-authorization"),
        Some("Basic PROXY-SECRET")
    );
    assert_eq!(request.header("x-plain"), Some("plain-value"));
}

#[tokio::test]
async fn bearer_token_reaches_the_server() {
    let server = serve(None, |_| Reply::ok("")).await;
    let credentials = HttpCredentials::Bearer(Secret::from(TOKEN.to_owned()));
    let (config, secrets) = input(server.url("/"), credentials).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env(),
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(
        server.seen()[0].header("authorization"),
        Some(format!("Bearer {TOKEN}").as_str())
    );
}

#[tokio::test]
async fn failure_messages_hold_no_secret_header_or_body() {
    let server = serve(None, |_| Reply::status(500, "BODY-TEXT sid=COOKIE-VALUE")).await;
    let (config, secrets) = input(server.url("/"), basic()).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env(),
    )
    .await;
    assert!(!result.ok);
    let message = result.message.unwrap();
    for secret in SECRETS.iter().chain(&["BODY-TEXT", "alice", "plain-value"]) {
        assert!(!message.contains(secret), "{secret} in {message}");
    }
}

#[tokio::test]
async fn a_missing_secret_fails_the_probe_without_sending_a_request() {
    let server = serve(None, |_| Reply::ok("")).await;
    let (config, _) = input(server.url("/"), basic()).split().unwrap();
    let result = probe(&target("x"), &CheckConfig::Http(config), None, &env()).await;
    assert_eq!(
        result.message.as_deref(),
        Some("a secret of the configuration is missing")
    );
    assert!(server.seen().is_empty());
}

#[tokio::test]
async fn credentials_are_not_sent_to_another_origin_on_redirect() {
    let other = serve(None, |_| Reply::ok("")).await;
    let to = other.url("/");
    let server = serve(None, move |_| Reply::redirect(&to)).await;
    let credentials = HttpCredentials::Bearer(Secret::from(TOKEN.to_owned()));
    let (config, secrets) = input(server.url("/"), credentials).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env(),
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    let followed = &other.seen()[0];
    assert_eq!(followed.header("authorization"), None);
    assert_eq!(followed.header("cookie"), None);
    assert_eq!(followed.header("proxy-authorization"), None);
}

#[test]
fn messages_lose_control_characters_and_stop_at_512_characters() {
    use crate::checks::probe::{MESSAGE_MAX, clean_message};
    assert_eq!(clean_message("a\r\nb\0c\u{1b}[31md\te"), "abc[31mde");
    let long = "x".repeat(MESSAGE_MAX + 100);
    assert_eq!(clean_message(&long).chars().count(), 512);
    assert_eq!(clean_message(&"y".repeat(512)).chars().count(), 512);
    assert_eq!(clean_message("short"), "short");
}

#[test]
fn log_007_messages_lose_bidi_zero_width_and_separator_characters() {
    use crate::checks::probe::clean_message;
    let text = "ok\u{202E}gnp.exe\u{2028}line\u{2029}para\u{200B}zw\u{FEFF}bom\u{061C}alm";
    assert_eq!(clean_message(text), "okgnp.exelineparazwbomalm");
    for c in [
        '\u{200B}', '\u{200F}', '\u{202A}', '\u{2060}', '\u{2069}', '\u{FEFF}', '\u{061C}',
    ] {
        assert_eq!(clean_message(&format!("a{c}b")), "ab", "{c:?}");
    }
    assert_eq!(
        clean_message("plain \u{e4}\u{4e2d}"),
        "plain \u{e4}\u{4e2d}"
    );
}

#[test]
fn split_refuses_secrets_that_are_not_utf8_and_names_the_field() {
    let bad = || Secret::new(vec![0xff, 0xfe, 0x41]);
    let mut headers = input("http://a.test/".into(), HttpCredentials::None);
    headers.headers.push(("Authorization".into(), bad()));
    assert_eq!(
        headers.split().err().unwrap().to_string(),
        "headers: is not valid"
    );
    let mut plain = input("http://a.test/".into(), HttpCredentials::None);
    plain.headers.push(("X-Other".into(), bad()));
    assert_eq!(plain.split().err().unwrap().field, "headers");
    let password = HttpCredentials::Basic {
        username: "u".into(),
        password: bad(),
    };
    assert_eq!(
        input("http://a.test/".into(), password)
            .split()
            .err()
            .unwrap()
            .field,
        "auth"
    );
    let token = HttpCredentials::Bearer(bad());
    assert_eq!(
        input("http://a.test/".into(), token)
            .split()
            .err()
            .unwrap()
            .field,
        "auth"
    );
}
