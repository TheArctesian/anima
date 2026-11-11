//! The recording catalogue through the hub's router (K1, RA-041, D-13, D-23, R-100): an API key
//! may list recordings but never open or delete one, and nothing changes when it tries; a
//! login token opens a `/recording` ticket and deletes the row and the file under the root
//! `ANIMA_RECORDING_ROOT` names. `Session.recording` is in `state`.

mod state;

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use anima_hub::{
    clock::{Clock, FixedClock},
    config,
    events::Bus,
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;
use tempfile::TempDir;

use super::{DOCKER_HOST, Row, insert, now, session_id};
use crate::hub::{Hub, dependencies, error, set_operator, start};

const RECORDING: &str = "0190a000-0000-7000-8030-000000000001";

const LIST: &str = "{ recordings { edges { node { id sessionId } } } }";
const OPEN: &str = "mutation($id: ID!) { openRecording(id: $id) { sessionId tunnelPath } }";
const DELETE: &str = "mutation($id: ID!) { deleteRecording(id: $id) }";

/// A hub at `start()` that records into a fresh folder, signed in by login, with one
/// recording of a `docker-host` session and its file.
struct Shelf {
    hub: Hub,
    login: String,
    root: TempDir,
}

impl Shelf {
    async fn new(pool: &PgPool) -> Self {
        set_operator(pool).await;
        let root = tempfile::tempdir().unwrap();
        let env = BTreeMap::from([(
            "ANIMA_RECORDING_ROOT".to_owned(),
            root.path().to_str().unwrap().to_owned(),
        )]);
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let mut serving = dependencies(pool, clock, Bus::new(16));
        serving.config = Arc::new(config::from_env(&env).unwrap());
        let hub = Hub::serving(pool, serving);
        let login = hub.token().await;
        insert(pool, &Row::live(1, DOCKER_HOST, 0)).await.unwrap();
        sqlx::query(
            "insert into sessions.recordings (id, session_id, device_id, device_name, protocol, \
                    format, key_sealed, started_at, expires_at) \
             values ($1::text::uuid, $2, $3::text::uuid, 'docker-host', 'ssh', 'asciicast', \
                     '\\x00', $4, $5)",
        )
        .bind(RECORDING)
        .bind(session_id(1))
        .bind(DOCKER_HOST)
        .bind(now())
        .bind(now() + TimeDelta::days(30))
        .execute(pool)
        .await
        .unwrap();
        let shelf = Self { hub, login, root };
        std::fs::write(shelf.file(), b"sealed").unwrap();
        shelf
    }

    fn file(&self) -> PathBuf {
        self.root.path().join(format!("{RECORDING}.anrec"))
    }

    /// An API key of `scope`, made with the login token.
    async fn key(&self, scope: &str) -> String {
        let document = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
        let input =
            json!({ "name": scope, "scope": scope, "expiresAt": start() + TimeDelta::days(30) });
        let made = self
            .hub
            .post(Some(&self.login), document, json!({ "input": input }))
            .await;
        made["data"]["createApiKey"]["secret"]
            .as_str()
            .unwrap_or_else(|| panic!("{made}"))
            .to_owned()
    }

    async fn post(&self, token: &str, document: &str) -> Value {
        self.hub
            .post(Some(token), document, json!({ "id": RECORDING }))
            .await
    }
}

async fn recording_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from sessions.recordings")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Audit entries about recordings.
async fn recording_audits(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("select action from events.audit_log where target_type = 'recording'")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn d_23_an_api_key_lists_recordings_but_never_opens_or_deletes_one(pool: PgPool) {
    let shelf = Shelf::new(&pool).await;
    for scope in ["READ", "WRITE"] {
        let key = shelf.key(scope).await;
        let listed = shelf.post(&key, LIST).await;
        assert_eq!(
            listed["data"]["recordings"]["edges"][0]["node"]["id"], RECORDING,
            "{scope}: {listed}"
        );
        for document in [OPEN, DELETE] {
            let refused = shelf.post(&key, document).await;
            let (_, code) = error(&refused);
            assert_eq!(code, "FORBIDDEN", "{scope} {document}");
            assert_eq!(refused["data"], Value::Null);
        }
    }
    assert_eq!(recording_rows(&pool).await, 1);
    assert!(shelf.file().exists());
    assert_eq!(recording_audits(&pool).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_041_a_login_opens_then_deletes_a_recording_under_the_configured_root(pool: PgPool) {
    let shelf = Shelf::new(&pool).await;
    let opened = shelf.post(&shelf.login, OPEN).await;
    assert_eq!(
        opened["data"]["openRecording"],
        json!({ "sessionId": session_id(1).to_string(), "tunnelPath": "/recording" })
    );
    let deleted = shelf.post(&shelf.login, DELETE).await;
    assert_eq!(deleted["data"]["deleteRecording"], RECORDING, "{deleted}");
    assert_eq!(recording_rows(&pool).await, 0);
    assert!(!shelf.file().exists(), "removed from ANIMA_RECORDING_ROOT");
    assert_eq!(recording_audits(&pool).await, ["view", "delete"]);
}
