//! The operator's secrets: a password typed right and wrong, a browser sign-in and its cookie
//! (G-14), a password in a URL, and a TOTP secret, code and replayed code (SEC-002, D-3, D-8,
//! LOG-007).

use anima_hub::access::{self, Password};
use axum::{body::Body, http::Request};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    Secrets,
    edge::{PUBLIC, WholeHub, code},
    forms::hex,
};
use crate::{
    hub::{USERNAME, test_vault},
    totp,
};

const LOGIN: &str = "mutation($input: LoginInput!) { login(input: $input) { token } }";
const SIGN_IN_BROWSER: &str =
    "mutation($input: LoginInput!) { signInBrowser(input: $input) { expiresAt } }";

fn input(password: &str, totp_code: Option<&str>) -> Value {
    json!({ "input": { "username": USERNAME, "password": password, "totpCode": totp_code } })
}

/// Makes the operator with a canary password, signs a browser in by cookie, enrols TOTP and
/// signs in with a code; returns the login token.
pub async fn sign_in(
    hub: &WholeHub,
    pool: &PgPool,
    now: DateTime<Utc>,
    secrets: &mut Secrets,
) -> String {
    let password = secrets.plant("password");
    access::set_operator(pool, USERNAME, Password::new(password.clone()), now)
        .await
        .unwrap();
    let wrong = secrets.plant("wrong-password");
    let refused = hub.graphql(&[], LOGIN, input(&wrong, None)).await;
    assert_eq!(code(&refused), "UNAUTHENTICATED");
    password_in_url(hub, secrets).await;
    browser(hub, &password, secrets).await;

    let enrolment = access::enrol_totp(pool, &test_vault(), USERNAME)
        .await
        .unwrap();
    let encoded = enrolment.uri().split("secret=").nth(1).unwrap();
    let encoded = encoded.split('&').next().unwrap();
    secrets.keep("totp-secret", encoded);
    let secret = base32(encoded);
    secrets.keep(
        "totp-secret-hex",
        &secret.iter().copied().fold(String::new(), hex),
    );
    let typed = totp::code(&secret, now);
    secrets.keep_code("totp-code", &typed);
    let signed_in = hub
        .graphql(&[], LOGIN, input(&password, Some(&typed)))
        .await;
    let token = signed_in["data"]["login"]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    secrets.keep("login-token", &token);
    let replayed = hub
        .graphql(&[], LOGIN, input(&password, Some(&typed)))
        .await;
    assert_eq!(code(&replayed), "UNAUTHENTICATED", "a code signs in once");
    let me = hub.data(&token, "{ me { username } }", json!({})).await;
    assert_eq!(me["me"]["username"], USERNAME);
    token
}

/// Signs a browser in from the public origin and reads `me` by its cookie.
async fn browser(hub: &WholeHub, password: &str, secrets: &mut Secrets) {
    let origin = [("origin", PUBLIC)];
    let answer = (hub.graphql_answer(&origin, SIGN_IN_BROWSER, input(password, None))).await;
    assert!(
        answer.json()["data"]["signInBrowser"].is_object(),
        "signed in"
    );
    let [set_cookie] = answer.set_cookies.try_into().expect("one Set-Cookie");
    let value = set_cookie
        .strip_prefix("__Host-anima=")
        .expect("the cookie");
    let token = value.split(';').next().unwrap();
    secrets.keep("browser-cookie", token);
    let cookie = format!("__Host-anima={token}");
    let headers = [("origin", PUBLIC), ("cookie", cookie.as_str())];
    let me = hub
        .graphql(&headers, "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["data"]["me"]["username"], USERNAME);
    // The cookie from another origin is refused: the error path of the same value.
    let elsewhere = [("origin", "https://elsewhere.example"), ("cookie", &cookie)];
    let refused = hub
        .graphql(&elsewhere, "{ me { username } }", json!({}))
        .await;
    assert_eq!(code(&refused), "UNAUTHENTICATED");
}

/// A password in the query string of `GET /graphql`, as a client that sends GraphQL over GET
/// would put it: the hub refuses the request and logs its route, never its query.
async fn password_in_url(hub: &WholeHub, secrets: &mut Secrets) {
    let password = secrets.plant("url-password");
    let variables = format!("%7B%22password%22%3A%22{password}%22%7D");
    let uri = format!("/graphql?query=mutation&variables={variables}");
    let answer = hub
        .send(Request::get(uri).body(Body::empty()).unwrap())
        .await;
    assert!(answer.status.is_client_error(), "{}", answer.status);
}

/// RFC 4648 base32 without padding, as an `otpauth://` URI carries the secret.
fn base32(encoded: &str) -> Vec<u8> {
    let (mut buffer, mut bits, mut out) = (0_u32, 0, Vec::new());
    for c in encoded.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => panic!("not base32"),
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).unwrap());
        }
    }
    out
}
