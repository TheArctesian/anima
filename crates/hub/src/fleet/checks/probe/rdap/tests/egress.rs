//! RDAP egress (P3-003, P-6, D-29): HTTPS only, each hop a new checked target, at most 3
//! redirects, TLS verified, the body capped. Local servers stand in for the registries; the
//! test policy allows loopback.

use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use ipnet::IpNet;

use super::server::{Answer, redirect, reply, serve, serve_on};
use crate::fleet::{
    checks::probe::rdap::egress::{Egress, FetchError, REDIRECTS_MAX, classify},
    targets::{AddressPolicy, TargetError},
};

const CAP: usize = 1024;

fn local(root: &[u8]) -> Egress {
    Egress::new(AddressPolicy::default().allowing_loopback()).trusting(root.to_vec())
}

/// Each `/hop/<n>` redirects to `/hop/<n + 1>` until `/hop/<last>`, which answers.
fn chain(last: usize) -> impl Fn(&str, u16) -> Vec<u8> + Send + Sync {
    move |path, port| {
        let n: usize = path.trim_start_matches("/hop/").parse().unwrap_or(0);
        if n < last {
            redirect(&format!("https://localhost:{port}/hop/{}", n + 1))
        } else {
            reply("200 OK", b"{}")
        }
    }
}

#[tokio::test]
async fn p3_003_an_https_target_is_read() {
    let server = serve(|_, _| reply("200 OK", b"{\"events\":[]}")).await;

    let body = local(&server.root)
        .get(&server.url("/domain/a.test"), CAP)
        .await;

    assert_eq!(body.unwrap(), b"{\"events\":[]}");
    assert_eq!(server.lines(), ["GET /domain/a.test HTTP/1.1"]);
}

#[tokio::test]
async fn p_6_an_http_target_is_refused_with_no_connection() {
    let server = serve(|_, _| reply("200 OK", b"{}")).await;
    let plain = reqwest::Url::parse(&format!("http://localhost:{}/", server.port)).unwrap();
    let with_user =
        reqwest::Url::parse(&format!("https://user@localhost:{}/", server.port)).unwrap();

    let egress = local(&server.root);
    assert_eq!(egress.get(&plain, CAP).await, Err(FetchError::Address));
    assert_eq!(egress.get(&with_user, CAP).await, Err(FetchError::Address));
    assert_eq!(server.connections().await, 0);
}

#[tokio::test]
async fn p_6_a_redirect_to_http_is_refused() {
    let server = serve(|path, port| match path {
        "/" => redirect(&format!("http://localhost:{port}/plain")),
        _ => reply("200 OK", b"{}"),
    })
    .await;

    let fetched = local(&server.root).get(&server.url("/"), CAP).await;

    assert_eq!(fetched, Err(FetchError::Address));
    assert_eq!(server.lines(), ["GET / HTTP/1.1"]);
    assert_eq!(server.connections().await, 1);
}

#[tokio::test]
async fn p3_003_a_redirect_to_a_denied_address_is_refused_with_no_connection() {
    // The second hop names ::1, which the deny list covers while the first hop's 127.0.0.1 stays
    // allowed; the server there would answer, so a connection would show in its lines. ::1, not
    // a second 127/8 address, because macOS binds only 127.0.0.1 on loopback.
    let answer: Answer = Arc::new(|_, _| reply("200 OK", b"{}"));
    let denied = serve_on(IpAddr::from(Ipv6Addr::LOCALHOST), answer).await;
    let target = denied.port;
    let first = serve(move |_, _| redirect(&format!("https://[::1]:{target}/"))).await;
    let deny: Vec<IpNet> = vec!["::1/128".parse().unwrap()];
    let egress =
        Egress::new(AddressPolicy::new(deny).allowing_loopback()).trusting(first.root.clone());

    // The first hop by its IPv4 address: `localhost` also resolves to the denied ::1.
    let start = reqwest::Url::parse(&format!("https://127.0.0.1:{}/", first.port)).unwrap();
    let fetched = egress.get(&start, CAP).await;

    assert_eq!(fetched, Err(FetchError::Target(TargetError::Refused)));
    assert_eq!(first.lines(), ["GET / HTTP/1.1"]);
    assert_eq!(denied.connections().await, 0);
}

#[tokio::test]
async fn p3_003_a_redirect_to_loopback_is_refused_under_the_default_policy() {
    // The default policy refuses loopback, so the hub cannot even reach the first hop here; a
    // redirect named by a registry is checked by the same rule.
    let server = serve(|_, _| reply("200 OK", b"{}")).await;
    let egress = Egress::new(AddressPolicy::default()).trusting(server.root.clone());

    let fetched = egress.get(&server.url("/"), CAP).await;

    assert_eq!(fetched, Err(FetchError::Target(TargetError::Refused)));
    assert_eq!(server.connections().await, 0);
}

#[tokio::test]
async fn d_29_three_redirects_are_followed_and_a_fourth_is_refused() {
    let three = serve(chain(REDIRECTS_MAX)).await;
    let four = serve(chain(REDIRECTS_MAX + 1)).await;

    let followed = local(&three.root).get(&three.url("/hop/0"), CAP).await;
    let refused = local(&four.root).get(&four.url("/hop/0"), CAP).await;

    assert_eq!(followed.unwrap(), b"{}");
    assert_eq!(three.lines().len(), REDIRECTS_MAX + 1);
    assert_eq!(refused, Err(FetchError::Redirects));
    assert_eq!(
        four.lines().len(),
        REDIRECTS_MAX + 1,
        "the fourth redirect is not followed"
    );
}

#[tokio::test]
async fn p_6_a_body_over_the_cap_is_refused() {
    let declared = serve(|_, _| reply("200 OK", &[b' '; CAP + 1])).await;
    let undeclared = serve(|_, _| {
        let mut bytes = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        bytes.extend_from_slice(&[b' '; CAP + 1]);
        bytes
    })
    .await;
    let exact = serve(|_, _| reply("200 OK", &[b' '; CAP])).await;

    let declared = local(&declared.root).get(&declared.url("/"), CAP).await;
    let undeclared = local(&undeclared.root).get(&undeclared.url("/"), CAP).await;
    let exact = local(&exact.root).get(&exact.url("/"), CAP).await;

    assert_eq!(declared, Err(FetchError::TooLong));
    assert_eq!(undeclared, Err(FetchError::TooLong));
    assert_eq!(exact.unwrap().len(), CAP);
}

#[tokio::test]
async fn p_6_an_untrusted_certificate_fails_before_the_request() {
    let server = serve(|_, _| reply("200 OK", b"{}")).await;
    let egress = Egress::new(AddressPolicy::default().allowing_loopback());

    let fetched = egress.get(&server.url("/"), CAP).await;

    assert_eq!(fetched, Err(FetchError::Tls));
    assert_eq!(server.connections().await, 1);
    assert_eq!(server.lines(), Vec::<String>::new());
}

#[tokio::test]
async fn p3_003_an_answer_outside_2xx_is_its_status() {
    let server = serve(|_, _| reply("404 Not Found", b"{}")).await;

    let fetched = local(&server.root).get(&server.url("/"), CAP).await;

    assert_eq!(fetched, Err(FetchError::Status(404)));
}

#[tokio::test]
async fn p3_003_the_pinned_client_resolves_no_name_of_its_own() {
    let server = serve(|_, _| reply("200 OK", b"{}")).await;
    let pinned: SocketAddr = ([127, 0, 0, 1], server.port).into();
    let client = local(&server.root)
        .pinned_client(Some("pinned.invalid"), &[pinned])
        .unwrap();
    let pinned_url = reqwest::Url::parse(&format!("https://pinned.invalid:{}/", server.port));

    // `.invalid` never resolves: the pinned name reaches the pinned address only because the
    // client looked nothing up, and `localhost`, which resolves, is not looked up at all.
    let pinned_reply = client.get(pinned_url.unwrap()).send().await.unwrap();
    let unpinned = client.get(server.url("/")).send().await.unwrap_err();

    assert_eq!(pinned_reply.status(), 200);
    assert_eq!(
        classify(&unpinned),
        FetchError::Target(TargetError::Unresolved)
    );
    assert_eq!(server.connections().await, 1);
}
