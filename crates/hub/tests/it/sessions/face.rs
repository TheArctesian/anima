//! The sessions face other domains and `main.rs` call: host keys, the stale-row close at
//! start and the shutdown.

use std::collections::BTreeMap;

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, ECDSA_KEY, ED25519, ED25519_FINGERPRINT, ED25519_KEY, OLLAMA, OPERATOR_ID, PVE_01,
    Row, add_operator,
    harness::{CLIENT_ADDR, schema_at},
    insert_all, now, put_host_key, session_id,
    starting::{end_audits, event_count, ready, redeem, session_audits, started, stored},
};
use anima_hub::{
    events::ClientAddr,
    ids::DeviceId,
    sessions::{self, EndReason, Error, Live, Tunnel},
};

fn device(id: &str) -> DeviceId {
    id.parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_host_key_is_none_until_one_is_accepted(pool: PgPool) {
    add_operator(&pool).await;
    assert!(
        sessions::host_key(&pool, device(PVE_01))
            .await
            .unwrap()
            .is_none()
    );

    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    let key = sessions::host_key(&pool, device(PVE_01))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(key.public_key.algorithm(), "ssh-ed25519");
    assert_eq!(key.public_key.fingerprint().as_str(), ED25519_FINGERPRINT);
    assert_eq!(key.public_key.to_base64(), ED25519);
    assert_eq!(key.first_seen_at, now());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_host_keys_returns_each_device_that_has_one(pool: PgPool) {
    add_operator(&pool).await;
    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    put_host_key(&pool, DOCKER_HOST, ECDSA_KEY).await.unwrap();

    let keys = sessions::host_keys(
        &pool,
        &[device(PVE_01), device(DOCKER_HOST), device(OLLAMA)],
    )
    .await
    .unwrap();
    let algorithms: BTreeMap<String, &str> = keys
        .iter()
        .map(|(id, key)| (id.to_string(), key.public_key.algorithm()))
        .collect();
    assert_eq!(
        algorithms,
        BTreeMap::from([
            (PVE_01.to_owned(), "ssh-ed25519"),
            (DOCKER_HOST.to_owned(), "ecdsa-sha2-nistp256"),
        ])
    );
    assert!(sessions::host_keys(&pool, &[]).await.unwrap().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_007_a_stored_key_whose_algorithm_differs_from_the_blob_is_an_error(pool: PgPool) {
    add_operator(&pool).await;
    // The table checks the name's shape only; the hub checks it names the key's algorithm.
    put_host_key(&pool, PVE_01, ("ssh-rsa", ED25519, ED25519_FINGERPRINT))
        .await
        .unwrap();
    let error = sessions::host_key(&pool, device(PVE_01)).await.unwrap_err();
    assert!(matches!(error, Error::Database(_)));
    assert_eq!(error.to_string(), "database error");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_close_stale_ends_only_open_sessions_with_hub_shutdown(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::live(2, DOCKER_HOST, 1),
            Row::ended(3, PVE_01, 2, "operator"),
        ],
    )
    .await;
    let at = now() + TimeDelta::hours(1);

    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 2);

    let rows: Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>, String)> =
        sqlx::query_as("select id, ended_at, end_reason from sessions.sessions order by id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            (session_id(1), at, "hub_shutdown".to_owned()),
            (session_id(2), at, "hub_shutdown".to_owned()),
            (
                session_id(3),
                now() + TimeDelta::minutes(7),
                "operator".to_owned()
            ),
        ]
    );
    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 0);
}

const ALICE: &str = "0190a000-0000-7000-8000-000000000002";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_005_close_stale_audits_each_row_it_ends_naming_its_operator_and_client(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::ended(2, PVE_01, 1, "operator"),
        ],
    )
    .await;
    // Another operator's session, from another address.
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at) \
         values ($1, $2::text::uuid, 'docker-host', 'ssh', $3::text::uuid, 'alice', \
                 '192.0.2.9', $4)",
    )
    .bind(session_id(3))
    .bind(DOCKER_HOST)
    .bind(ALICE)
    .bind(now())
    .execute(&pool)
    .await
    .unwrap();
    let at = now() + TimeDelta::hours(1);

    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 2);

    let audits: Vec<(String, String, String, String, String, Value)> = sqlx::query_as(
        "select target_id, operator_id::text, operator_name, host(client_addr), action, change \
         from events.audit_log where target_type = 'session' and at = $1 order by target_id",
    )
    .bind(at)
    .fetch_all(&pool)
    .await
    .unwrap();
    let entry = |n, operator: &str, name: &str, client: &str| {
        (
            session_id(n).to_string(),
            operator.to_owned(),
            name.to_owned(),
            client.to_owned(),
            "end".to_owned(),
            json!({ "reason": "hub_shutdown" }),
        )
    };
    assert_eq!(
        audits,
        [
            entry(1, OPERATOR_ID, "operator", "10.10.0.77"),
            entry(3, ALICE, "alice", "192.0.2.9"),
        ]
    );
    assert_eq!(
        event_count(&pool).await,
        0,
        "audit only: no tunnel ended (D-13)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn d_10_shut_down_ends_every_session_the_hub_holds_hub_shutdown_once(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01, DOCKER_HOST, OLLAMA]).await;
    let pending = started(&schema, PVE_01).await;
    let spent = started(&schema, DOCKER_HOST).await;
    let elsewhere = ClientAddr("10.10.0.99".parse().unwrap());
    assert!(
        live.redeem(spent["token"].as_str().unwrap(), elsewhere, now())
            .is_err()
    );
    let opened = started(&schema, OLLAMA).await;
    let tunnel = redeem(&live, &opened, now()).unwrap().tunnel;
    let tunnel_task = tokio::spawn(stand_in_tunnel(pool.clone(), tunnel, id_of(&opened)));
    let at = now() + TimeDelta::seconds(5);

    sessions::shut_down(&pool, &live, at).await.unwrap();

    // The stand-in ends its row before it deregisters, so the row shows shut_down waited.
    let opened_row = stored(&pool, &id_of(&opened)).await;
    assert_eq!(opened_row.end_reason.as_deref(), Some("hub_shutdown"));
    assert_eq!(tunnel_task.await.unwrap(), EndReason::HubShutdown);
    for ticket in [&pending, &spent] {
        let row = stored(&pool, &id_of(ticket)).await;
        assert_eq!(row.ended_at, Some(at));
        assert_eq!(row.end_reason.as_deref(), Some("hub_shutdown"));
        let audits = end_audits(&pool, &id_of(ticket)).await;
        assert_eq!(audits.len(), 1, "{ticket}");
        assert_eq!(audits[0].change, json!({ "reason": "hub_shutdown" }));
        assert_eq!(audits[0].operator_name, "operator");
        assert_eq!(audits[0].client, CLIENT_ADDR);
    }
    assert!(
        end_audits(&pool, &id_of(&opened)).await.is_empty(),
        "the tunnel writes its own end entry"
    );
    assert_eq!(event_count(&pool).await, 0, "ticket ends are audit only");
    assert!(
        redeem(&live, &pending, at).is_none(),
        "the ticket is withdrawn"
    );

    // Shutting down again finds nothing left to end.
    sessions::shut_down(&pool, &live, at + TimeDelta::seconds(1))
        .await
        .unwrap();
    assert_eq!(
        session_audits(&pool).await.len(),
        5,
        "three starts, two ends"
    );
}

fn id_of(ticket: &Value) -> String {
    ticket["sessionId"].as_str().unwrap().to_owned()
}

/// A stand-in for the tunnel task: on the signal it ends its row, then deregisters.
async fn stand_in_tunnel(pool: PgPool, mut tunnel: Tunnel, id: String) -> EndReason {
    let cause = tunnel.ended().await;
    sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'hub_shutdown' \
         where id = $1::text::uuid and ended_at is null",
    )
    .bind(&id)
    .bind(now() + TimeDelta::seconds(5))
    .execute(&pool)
    .await
    .unwrap();
    drop(tunnel);
    cause.reason
}
