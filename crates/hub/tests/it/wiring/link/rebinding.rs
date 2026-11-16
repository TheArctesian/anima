//! What the link mode adds before sign-in (FRN-007, SEC-002): only a loopback host and no
//! origin or the app's origin reach a route, so a DNS-rebinding page cannot drive failed
//! sign-ins into this computer's throttle; and an operator without TOTP cannot sign in. The
//! VPS mode keeps neither rule.

use std::{net::SocketAddr, sync::Arc};

use anima_hub::{
    all_routes,
    clock::{Clock, FixedClock},
    fleet::{access, events::Bus},
};
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{From, LAPTOP, Post, graphql, login_request, post_with, serve_linked};
use crate::hub::{Hub, PASSWORD, USERNAME, dependencies, start};

const TYPENAME: &str = r#"{"query":"{ __typename }"}"#;

fn typename() -> Post<'static> {
    Post {
        path: "/graphql",
        token: None,
        body: TYPENAME,
    }
}

async fn attempts(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from access.login_attempts")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_007_a_request_that_names_no_loopback_host_is_refused_bare(pool: PgPool) {
    let hub = serve_linked(&pool).await;
    for host in [
        "rebind.example:8080",
        "127.0.0.1.rebind.example",
        "localhost.rebind.example",
        "127.0.0.2",
        "[::2]:80",
        "localhost:",
        "localhost:http",
    ] {
        let headers = format!("host: {host}\r\n");
        let answer = post_with(&hub, From::ThisComputer, typename(), &headers).await;
        assert!(answer.starts_with("HTTP/1.1 403"), "{host}: {answer}");
        assert!(answer.ends_with("\r\n\r\n"), "no body for {host}: {answer}");
    }
    for host in ["127.0.0.1", "localhost:1", "LOCALHOST", "[::1]:65535"] {
        let headers = format!("host: {host}\r\n");
        let answer = post_with(&hub, From::ThisComputer, typename(), &headers).await;
        assert!(answer.starts_with("HTTP/1.1 200"), "{host}: {answer}");
    }
    let device = post_with(
        &hub,
        From::Device(LAPTOP),
        typename(),
        "host: rebind.example\r\n",
    );
    assert!(device.await.starts_with("HTTP/1.1 403"));
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_007_a_page_origin_is_refused_before_its_sign_in_counts(pool: PgPool) {
    let hub = serve_linked(&pool).await;
    let body = login_request(None).to_string();
    let sign_in = Post {
        path: "/graphql",
        token: None,
        body: &body,
    };
    for origin in ["http://127.0.0.1:8080", "http://rebind.example", "null"] {
        let headers = format!("host: 127.0.0.1\r\norigin: {origin}\r\n");
        let answer = post_with(&hub, From::ThisComputer, sign_in, &headers).await;
        assert!(answer.starts_with("HTTP/1.1 403"), "{origin}: {answer}");
    }
    assert_eq!(
        attempts(&pool).await,
        0,
        "no refused request reached sign-in"
    );

    let headers = "host: 127.0.0.1\r\norigin: tauri://localhost\r\n";
    let app = post_with(&hub, From::ThisComputer, typename(), headers).await;
    assert!(app.starts_with("HTTP/1.1 200"), "{app}");
}

fn refused_like_a_wrong_password(response: &Value) -> bool {
    response["data"].is_null()
        && response["errors"][0]["extensions"]["code"] == "UNAUTHENTICATED"
        && response["errors"][0]["message"] == "The username or password is wrong."
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_007_sec_002_the_link_mode_refuses_an_operator_without_totp(pool: PgPool) {
    let hub = serve_linked(&pool).await;
    access::disable_totp(&pool, USERNAME).await.unwrap();

    let refused = graphql(&hub, From::ThisComputer, None, login_request(None)).await;
    assert!(refused_like_a_wrong_password(&refused), "{refused}");
    let with_a_code = login_request(Some("123456"));
    let refused = graphql(&hub, From::Device(LAPTOP), None, with_a_code).await;
    assert!(refused_like_a_wrong_password(&refused), "{refused}");

    // Outside the link mode the same account signs in with its password alone.
    let vps = Hub::new(&pool, start());
    assert!(vps.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_007_the_vps_mode_answers_any_host_and_origin(pool: PgPool) {
    let networks = ["127.0.0.0/8".parse().unwrap()];
    access::permit_networks(&pool, &networks).await.unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let bus = Bus::new(16);
    let edge = access::Edge::new(pool.clone(), bus.clone(), clock.clone());
    let app = all_routes(dependencies(&pool, clock, bus), edge);
    let mut request = Request::post("/graphql")
        .header(header::HOST, "rebind.example")
        .header(header::ORIGIN, "http://rebind.example")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(TYPENAME))
        .unwrap();
    let peer: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
