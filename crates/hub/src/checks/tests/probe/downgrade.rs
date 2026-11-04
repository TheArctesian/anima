//! A redirect from HTTPS to HTTP is refused while secrets are attached (SEC-005).

use std::sync::atomic::Ordering;

use super::support::{LOOPBACK, Reply, Server, acceptor, env, serve, target};
use crate::checks::{
    config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, StatusRange},
    probe::{ProbeEnv, probe},
};
use crate::credentials::Secret;

fn input(url: String, credentials: HttpCredentials) -> HttpInput {
    HttpInput {
        url,
        method: HttpMethod::Get,
        headers: Vec::new(),
        body: None,
        max_redirects: 3,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials,
        verify_tls: false,
        timeout_ms: 3000,
    }
}

/// An HTTPS server that redirects every request to a plain HTTP server, and the plain one.
async fn servers() -> (Server, Server, ProbeEnv) {
    let plain = serve(None, |_| Reply::ok("plain")).await;
    let to = format!("http://localhost:{}/", plain.addr.port());
    let secure = serve(Some(acceptor(2020, 2040)), move |_| Reply::redirect(&to)).await;
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("localhost".to_owned(), vec![LOOPBACK])]).into();
    (secure, plain, env)
}

#[tokio::test]
async fn sec_005_secrets_never_follow_a_redirect_from_https_to_http() {
    let (secure, plain, env) = servers().await;
    let url = format!("https://localhost:{}/", secure.addr.port());
    let credentials = HttpCredentials::Bearer(Secret::from("tok-DOWNGRADE".to_owned()));
    let (config, secrets) = input(url, credentials).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env,
    )
    .await;
    assert!(!result.ok);
    assert_eq!(
        result.message.as_deref(),
        Some("a redirect left HTTPS while secrets are attached")
    );
    assert_eq!(secure.seen().len(), 1);
    assert_eq!(plain.connections.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn sec_005_a_check_without_secrets_may_follow_a_redirect_to_http() {
    let (secure, plain, env) = servers().await;
    let url = format!("https://localhost:{}/", secure.addr.port());
    let (config, secrets) = input(url, HttpCredentials::None).split().unwrap();
    let result = probe(
        &target("x"),
        &CheckConfig::Http(config),
        Some(&secrets),
        &env,
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(plain.seen().len(), 1);
}
