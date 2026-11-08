//! MET-013, K8 Section 2: an agent token or an enrolment code is no sign-in anywhere but the
//! agent routes: `/graphql` over HTTP and the WebSocket refuse both. An API key may not make a
//! code or revoke a token (INT-003, D-23).

use std::{net::SocketAddr, sync::Arc};

use anima_hub::clock::Clock;
use async_graphql::futures_util::{SinkExt as _, StreamExt as _};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest as _, http::HeaderValue},
};

use super::harness::{Agents, PVE_01, app};
use crate::hub::{error, start};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_an_agent_token_or_code_is_no_graphql_sign_in(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let code = agents.code_for(PVE_01).await;
    for bearer in [&token, &code] {
        let me = agents
            .graphql(Some(bearer), "{ me { username } }", json!({}))
            .await;
        assert_eq!(error(&me).1, "UNAUTHENTICATED", "{me}");
    }
}

/// What the hub answers to a `connection_init` with `payload` on `/graphql`.
async fn socket_answer(addr: SocketAddr, payload: Value) -> Message {
    let mut request = format!("ws://{addr}/graphql")
        .into_client_request()
        .unwrap();
    let protocol = HeaderValue::from_static("graphql-transport-ws");
    request
        .headers_mut()
        .insert("sec-websocket-protocol", protocol);
    let (mut socket, _) = connect_async(request).await.unwrap();
    let init = json!({ "type": "connection_init", "payload": payload });
    socket.send(Message::text(init.to_string())).await.unwrap();
    let answer = tokio::time::timeout(std::time::Duration::from_secs(10), socket.next()).await;
    answer.unwrap().unwrap().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_an_agent_token_is_refused_on_the_websocket(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let clock: Arc<dyn Clock> = agents.clock.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service = app(&pool, clock).into_make_service_with_connect_info::<SocketAddr>();
    let served = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });

    let refused = socket_answer(addr, json!({ "authorization": format!("Bearer {token}") })).await;
    let Message::Close(Some(frame)) = refused else {
        panic!("not closed: {refused:?}");
    };
    assert_eq!(u16::from(frame.code), 4403);
    // The operator's token on the same socket route is accepted.
    let bearer = format!("Bearer {}", agents.operator);
    let accepted = socket_answer(addr, json!({ "authorization": bearer })).await;
    assert_eq!(accepted, Message::text(r#"{"type":"connection_ack"}"#));
    served.abort();
}

/// A WRITE API key of the operator, as WP-43's `createApiKey` stores one.
async fn api_key(pool: &PgPool) -> String {
    sqlx::query(
        "INSERT INTO access.api_tokens \
         (id, operator_id, token_hash, kind, name, display_prefix, scope, expires_at, created_at) \
         SELECT gen_random_uuid(), id, $1, 'api_key', 'script', 'anima_CQkJCQ', 'write', $2, $3 \
         FROM access.operators",
    )
    .bind(Sha256::digest([9_u8; 32]).to_vec())
    .bind(start() + chrono::TimeDelta::days(30))
    .bind(start())
    .execute(pool)
    .await
    .unwrap();
    format!("anima_{}", URL_SAFE_NO_PAD.encode([9_u8; 32]))
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
#[ignore = "needs WP-43 T3 deny list (D-23): enrolAgent and revokeAgentToken are login or browser token only"]
async fn int_003_an_api_key_may_not_enrol_or_revoke_an_agent(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let key = api_key(&pool).await;
    for mutation in [
        "mutation($id: ID!) { enrolAgent(deviceId: $id) { code } }",
        "mutation($id: ID!) { revokeAgentToken(deviceId: $id) { id } }",
    ] {
        let answer = agents
            .graphql(Some(&key), mutation, json!({ "id": PVE_01 }))
            .await;
        assert_eq!(error(&answer).1, "FORBIDDEN", "{answer}");
    }
}
