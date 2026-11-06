//! Container and proxy peers (`172.16.0.0/12`) are never clients, whatever the list says, and
//! only `GET /health` passes the layer unchecked (WP-04 deploy notes, NET-002).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;

use super::{assert_forbidden, edge, events, get_path, typename};

fn untrusted_refusal(client: &str) -> (String, String, String) {
    (
        "warning".to_owned(),
        "access".to_owned(),
        format!(
            "Refused a request from {client}: a container or proxy address (172.16.0.0/12) is \
             never taken as a client address."
        ),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn h_2_a_container_or_proxy_peer_is_refused_even_inside_a_permitted_network(pool: PgPool) {
    let edge = edge(&pool, &["0.0.0.0/0", "::/0"]).await;
    assert_forbidden(&edge.send_from("172.30.0.1:40000", typename()).await);
    assert_forbidden(
        &edge
            .send_from("[::ffff:172.16.0.9]:40000", get_path("/client"))
            .await,
    );
    assert_forbidden(
        &edge
            .send_from("172.31.255.255:40000", get_path("/client"))
            .await,
    );
    assert_eq!(
        events(&pool).await,
        vec![
            untrusted_refusal("172.30.0.1"),
            untrusted_refusal("172.16.0.9"),
            untrusted_refusal("172.31.255.255"),
        ]
    );
    let (status, _) = edge
        .send_from("172.32.0.1:40000", get_path("/client"))
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "172.32.0.1 is outside 172.16.0.0/12"
    );
    let (status, _) = edge
        .send_from("172.15.255.255:40000", get_path("/client"))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn h_2_only_get_health_passes_the_layer_unchecked(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    for peer in ["172.30.0.10:40000", "203.0.113.9:40000"] {
        let (status, body) = edge.send_from(peer, get_path("/health")).await;
        assert_ne!(status, StatusCode::FORBIDDEN, "{peer}: {body}");
    }
    let post = Request::post("/health").body(Body::empty()).unwrap();
    assert_forbidden(&edge.send_from("203.0.113.9:40000", post).await);
    assert_forbidden(
        &edge
            .send_from("203.0.113.9:40000", get_path("/health/x"))
            .await,
    );
    assert_forbidden(
        &edge
            .send_from("203.0.113.9:40000", get_path("/healthz"))
            .await,
    );
    let (status, _) = edge.send(get_path("/health")).await;
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "a peer address is still required"
    );
}
