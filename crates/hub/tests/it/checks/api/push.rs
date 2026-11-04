//! The push route: the token is the only authentication (CHK-003, CHK-005, LOG-007, R-028).

use std::sync::Arc;

use anima_hub::{
    checks::push_router,
    clock::Clock,
    events::{Bus, Signal},
    status::Status,
};
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt as _;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::harness::{
    PVE_01, StepClock, TestSchema, assert_absent, captured, create, data_with, database_now, input,
    schema_at,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

struct Rig {
    pool: PgPool,
    schema: TestSchema,
    bus: Bus,
    router: Router,
}

async fn rig(pool: PgPool) -> Rig {
    let at = database_now(&pool).await;
    let (schema, bus) = schema_at(&pool, at);
    let clock: Arc<dyn Clock> = Arc::new(StepClock::from(at));
    let router = push_router(pool.clone(), bus.clone(), clock);
    Rig {
        pool,
        schema,
        bus,
        router,
    }
}

impl Rig {
    /// A push check with no retries, so one `down` report is Down at once. Returns the id and
    /// its path.
    async fn push_check(&self, name: &str) -> (String, String) {
        let mut new = input(PVE_01, name, json!({ "push": true }));
        new["retries"] = json!(0);
        let id = create(&self.schema, new).await;
        let query =
            "query($id: ID!) { check(id: $id) { config { ... on PushConfig { pushPath } } } }";
        let read = data_with(&self.schema, query, json!({ "id": id })).await;
        let path = read["check"]["config"]["pushPath"]
            .as_str()
            .unwrap()
            .to_owned();
        (id, path)
    }

    async fn call(&self, method: Method, uri: &str, body: Vec<u8>) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::from(body))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, bytes.to_vec())
    }

    async fn get(&self, uri: &str) -> (StatusCode, Vec<u8>) {
        self.call(Method::GET, uri, Vec::new()).await
    }

    async fn beats(&self, id: &str) -> Vec<(String, Option<i32>, Option<String>)> {
        sqlx::query_as(
            "SELECT status, response_time_ms, message FROM checks.heartbeats \
             WHERE check_id = $1::text::uuid ORDER BY at",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

fixture_test! {
    async fn chk_003_a_valid_token_records_an_up_beat_and_publishes_only_the_heartbeat(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let mut receiver = rig.bus.subscribe();
        let (status, body) = rig.get(&format!("{path}?ping=42&msg=nightly%20done")).await;
        assert_eq!((status, body.as_slice()), (StatusCode::OK, b"ok".as_slice()));
        assert_eq!(rig.beats(&id).await, [("up".to_owned(), Some(42), Some("nightly done".to_owned()))]);
        let state: (String, bool) = sqlx::query_as(
            "SELECT status, next_due_at > last_beat_at FROM checks.checks WHERE id = $1::text::uuid",
        )
        .bind(&id)
        .fetch_one(&rig.pool)
        .await
        .unwrap();
        assert_eq!(state, ("up".to_owned(), true));
        match receiver.try_recv() {
            Ok(Signal::Heartbeat { beat, .. }) => {
                assert_eq!((beat.status, beat.response_time_ms), (Status::Up, Some(42)));
            }
            other => panic!("expected a heartbeat signal, got {other:?}"),
        }
        assert!(receiver.try_recv().is_err(), "the scheduler publishes status changes, not push");
    }

    async fn chk_003_post_works_like_get_and_needs_no_sign_in(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let (status, _) = rig.call(Method::POST, &path, b"anything".to_vec()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rig.beats(&id).await.len(), 1);
    }

    async fn chk_003_status_down_with_no_retries_records_a_down_beat(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let (status, _) = rig.get(&format!("{path}?status=down&msg=disk%20full&ping=5")).await;
        assert_eq!(status, StatusCode::OK);
        // A failure has no response time (the beat keeps it only when answered).
        assert_eq!(rig.beats(&id).await, [("down".to_owned(), None, Some("disk full".to_owned()))]);
    }

    async fn chk_003_an_unknown_token_or_a_token_one_character_off_is_a_bare_404(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let last = path.chars().last().unwrap();
        let swapped = if last == 'A' { 'B' } else { 'A' };
        let off = format!("{}{swapped}", &path[..path.len() - 1]);
        for uri in [off.as_str(), "/push/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"] {
            let (status, body) = rig.get(uri).await;
            assert_eq!((status, body.len()), (StatusCode::NOT_FOUND, 0), "{uri}");
        }
        assert!(rig.beats(&id).await.is_empty());
    }

    async fn chk_003_a_token_of_the_wrong_shape_is_a_bare_404_before_any_lookup(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let token = path.strip_prefix("/push/").unwrap();
        let malformed = [
            format!("/push/{}", &token[..42]),
            format!("/push/{token}A"),
            format!("/push/{}!", &token[..42]),
            format!("/push/{}=", &token[..42]),
            "/push/".to_owned(),
            format!("/push/{token}/extra"),
        ];
        for uri in &malformed {
            let (status, body) = rig.get(uri).await;
            assert_eq!((status, body.len()), (StatusCode::NOT_FOUND, 0), "{uri}");
        }
        assert!(rig.beats(&id).await.is_empty());
    }

    async fn chk_005_a_paused_check_answers_404_and_records_nothing_until_resumed(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let pause = "mutation($id: ID!, $paused: Boolean!) { setCheckPaused(id: $id, paused: $paused) { id } }";
        data_with(&rig.schema, pause, json!({ "id": id, "paused": true })).await;
        let (status, body) = rig.get(&path).await;
        assert_eq!((status, body.len()), (StatusCode::NOT_FOUND, 0));
        assert!(rig.beats(&id).await.is_empty());
        data_with(&rig.schema, pause, json!({ "id": id, "paused": false })).await;
        assert_eq!(rig.get(&path).await.0, StatusCode::OK);
    }

    async fn chk_003_the_message_is_cleaned_and_capped_at_512_characters(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let (status, _) = rig.get(&format!("{path}?msg=a%01b%0Ac%7Fd%E2%80%AEe")).await;
        assert_eq!(status, StatusCode::OK);
        let long = "x".repeat(600);
        assert_eq!(rig.get(&format!("{path}?msg={long}")).await.0, StatusCode::OK);
        let messages: Vec<Option<String>> = rig.beats(&id).await.into_iter().map(|b| b.2).collect();
        assert_eq!(messages[0].as_deref(), Some("abcde"));
        assert_eq!(messages[1].as_ref().map(String::len), Some(512));
    }

    async fn chk_003_a_status_or_ping_outside_the_contract_is_400_and_records_nothing(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let refused = ["status=sideways", "ping=600001", "ping=-1", "ping=1.5", "ping=abc", "status="];
        for query in refused {
            let (status, body) = rig.get(&format!("{path}?{query}")).await;
            assert_eq!((status, body.len()), (StatusCode::BAD_REQUEST, 0), "{query}");
        }
        assert!(rig.beats(&id).await.is_empty());
        assert_eq!(rig.get(&format!("{path}?ping=0")).await.0, StatusCode::OK);
        assert_eq!(rig.get(&format!("{path}?ping=600000")).await.0, StatusCode::OK);
    }

    async fn chk_003_a_body_over_1_kib_is_refused_and_one_of_1_kib_is_not(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check("backup").await;
        let (status, _) = rig.call(Method::POST, &path, vec![b'x'; 1025]).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(rig.beats(&id).await.is_empty());
        let (status, _) = rig.call(Method::POST, &path, vec![b'x'; 1024]).await;
        assert_eq!(status, StatusCode::OK);
    }

    async fn log_007_the_token_is_never_logged_not_even_when_the_write_fails(pool: PgPool) {
        let rig = rig(pool).await;
        let (_, path) = rig.push_check("backup").await;
        let token = path.strip_prefix("/push/").unwrap().to_owned();
        let (log, ok) = captured(|| rig.get(&path)).await;
        assert_eq!(ok.0, StatusCode::OK);
        let (unknown_log, _) = captured(|| rig.get("/push/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")).await;
        sqlx::query("DROP TABLE checks.heartbeats CASCADE").execute(&rig.pool).await.unwrap();
        let (failed_log, failed) = captured(|| rig.get(&path)).await;
        assert_eq!((failed.0, failed.1.len()), (StatusCode::INTERNAL_SERVER_ERROR, 0));
        assert!(failed_log.contains("could not be recorded"), "the failure is logged without detail");
        assert_absent(&token, &[&log, &unknown_log, &failed_log]);
        assert_absent("/push/", &[&log, &unknown_log, &failed_log]);
    }
}
