//! A fake control socket of `anima-link hub` in a private folder, and a signed-in hub in the
//! link mode that uses it.

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt as _,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anima_hub::{
    clock::{Clock, FixedClock},
    config,
    fleet::events::Bus,
    schema::Dependencies,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{UnixListener, UnixStream},
};

use crate::{
    hub::{Hub, PASSWORD, USERNAME, dependencies, start},
    totp::{code as totp_code, enrol},
};

pub const INVITE: &str = "anima-invite.1.ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
pub const KEY: &str = "nodekey:5c1a0000000000000000000000000000000000000000000000000000000000aa";

/// The `devices.list` reply line of one relayed device, `laptop`.
pub fn one_device() -> String {
    ok(&json!([{
        "key": KEY, "fingerprint": "1a2b3c4d", "name": "laptop",
        "address": "fd41:4e49:4d41::7", "permitted": "2030-05-05T12:00:00.123456789Z",
        "expires": "2030-08-03T12:00:00Z", "streams": 2, "path": "relayed", "relay": "derp-ams",
    }]))
}

pub fn ok(result: &Value) -> String {
    format!("{}\n", json!({ "ok": true, "result": result }))
}

pub fn refusal(code: &str) -> String {
    format!(
        "{}\n",
        json!({ "ok": false, "error": { "code": code, "message": "no" } })
    )
}

/// A fake control socket: it answers each request line with `answer(request)` and records
/// the requests.
pub struct FakeLink {
    _folder: tempfile::TempDir,
    pub path: PathBuf,
    seen: Arc<Mutex<Vec<Value>>>,
}

impl FakeLink {
    pub fn start(answer: impl Fn(&Value) -> String + Send + Sync + 'static) -> Self {
        Self::slow(Duration::ZERO, answer)
    }

    /// A link that waits `delay` before each answer.
    pub fn slow(
        delay: Duration,
        answer: impl Fn(&Value) -> String + Send + Sync + 'static,
    ) -> Self {
        let folder = tempfile::tempdir().unwrap();
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = folder.path().join("control.sock");
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (answer, record): (Answer, _) = (Arc::new(answer), seen.clone());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(stream, delay, answer.clone(), record.clone()));
            }
        });
        Self {
            _folder: folder,
            path,
            seen,
        }
    }

    /// Every request the link got.
    pub fn requests(&self) -> Vec<Value> {
        self.seen.lock().unwrap().clone()
    }

    /// The requests the link got, other than `devices.list`.
    pub fn changes(&self) -> Vec<Value> {
        (self.seen.lock().unwrap().iter())
            .filter(|request| request["verb"] != "devices.list")
            .cloned()
            .collect()
    }
}

type Answer = Arc<dyn Fn(&Value) -> String + Send + Sync>;

/// Answers each request line of one connection.
async fn serve(
    stream: UnixStream,
    delay: Duration,
    answer: Answer,
    record: Arc<Mutex<Vec<Value>>>,
) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let request: Value = serde_json::from_str(&line).unwrap();
        record.lock().unwrap().push(request.clone());
        tokio::time::sleep(delay).await;
        let _ = write.write_all(answer(&request).as_bytes()).await;
    }
}

/// A hub in the link mode whose control socket is `link`'s, with the operator's token.
pub async fn hub_over(pool: &PgPool, link: &FakeLink) -> (Hub, String) {
    let env = BTreeMap::from([
        (
            "ANIMA_LINK_SOCKET".to_owned(),
            "/run/anima/link.sock".to_owned(),
        ),
        (
            "ANIMA_LINK_CONTROL".to_owned(),
            link.path.display().to_string(),
        ),
    ]);
    // The link mode refuses a sign-in without TOTP (FRN-007).
    let secret = enrol(pool).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let parts = Dependencies {
        config: Arc::new(config::from_env(&env).unwrap()),
        ..dependencies(pool, clock, Bus::new(16))
    };
    let hub = Hub::serving(pool, parts);
    let token = signed_in(&hub, &totp_code(&secret, start())).await;
    (hub, token)
}

/// The operator's token from a sign-in with the password and `one_time_code`.
async fn signed_in(hub: &Hub, one_time_code: &str) -> String {
    let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
    let input = json!({ "username": USERNAME, "password": PASSWORD, "totpCode": one_time_code });
    let response = hub.post(None, query, json!({ "input": input })).await;
    response["data"]["login"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("login failed: {response}"))
        .to_owned()
}

/// An API key of `scope` made with the operator's `token`.
pub async fn api_key(hub: &Hub, token: &str, scope: &str) -> String {
    let query = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "name": format!("script-{scope}"), "scope": scope, "madeFor": "USER",
                        "expiresAt": start() + chrono::TimeDelta::days(30) });
    let made = hub
        .post(Some(token), query, json!({ "input": input }))
        .await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

/// A builder token enrolled with the operator's `token`.
pub async fn builder_token(hub: &Hub, token: &str) -> String {
    let query = "mutation { enrolBuilder(name: \"mini\", platform: { os: LINUX, arch: X86_64 }) \
                 { token } }";
    let made = hub.post(Some(token), query, json!({})).await;
    made["data"]["enrolBuilder"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

/// Runs `query` and returns its data; panics on errors.
pub async fn data(hub: &Hub, token: &str, query: &str, variables: Value) -> Value {
    let response = hub.post(Some(token), query, variables).await;
    assert!(response["errors"].is_null(), "{response}");
    response["data"].clone()
}

pub fn code(response: &Value) -> &Value {
    &response["errors"][0]["extensions"]["code"]
}

/// Every audit change and event message the hub wrote, as one text.
pub async fn written(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "select coalesce((select string_agg(change::text || ' ' || target_type || ' ' || action, ' ') \
         from events.audit_log), '') || ' ' || \
         coalesce((select string_agg(source || ' ' || message, ' ') from events.event_log), '')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Makes every later audit write of the test database fail, as a full disk would.
pub async fn break_the_audit_log(pool: &PgPool) {
    sqlx::query(
        "create function events.refuse_audit() returns trigger language plpgsql as \
         $$ begin raise exception 'audit refused'; end $$",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "create trigger refuse_audit before insert on events.audit_log \
         for each row execute function events.refuse_audit()",
    )
    .execute(pool)
    .await
    .unwrap();
}
