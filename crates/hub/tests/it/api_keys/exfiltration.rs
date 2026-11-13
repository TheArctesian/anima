//! The review's exfiltration chain with a WRITE key: move a device to a host the key holder
//! runs, accept that host's key, and test the connection so that the hub signs in there with
//! the stored password. An API key may neither move the device nor pin a host key, so the
//! chain stops at its first step and the stored host, its pin and its password stay put
//! (D-23, R-100, SEC-006).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        credentials::{Secret, SshSecret, store_ssh},
        events::Bus,
        sessions::Live,
    },
    ids::DeviceId,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::net::TcpListener;

use super::{kind_refused, rows, secret};
use crate::hub::{Hub, dependencies, error, host_of, set_operator, start, test_vault};

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

/// A real OpenSSH public key (throwaway; the private half was deleted) and the fingerprint
/// `ssh-keygen -l` printed for it: the pin of the device the operator trusts.
const PINNED: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
const PINNED_FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";

/// The fingerprint the key holder's own host shows.
const THEIRS: &str = "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

const MOVE: &str = "mutation($id: ID!, $input: UpdateDeviceInput!) { \
    updateDevice(id: $id, input: $input) { host } }";

const ACCEPT: &str = "mutation($id: ID!, $fingerprint: String!) { \
    acceptHostKey(deviceId: $id, fingerprintSha256: $fingerprint) { fingerprintSha256 } }";

const TEST: &str = "mutation($input: ConnectionTestInput!) { \
    testConnection(input: $input) { reachable authenticated } }";

/// A loopback listener that counts the connections it accepts and drops each one.
struct Listener {
    port: u16,
    accepted: Arc<AtomicUsize>,
}

impl Listener {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                drop(stream);
            }
        });
        Self { port, accepted }
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }
}

/// PVE-01 as the operator set it up: at `port` on the loopback interface, signing in with a
/// stored password, its host key pinned.
async fn trusted_device(pool: &PgPool, port: u16) {
    sqlx::query("update devices.devices set host = '127.0.0.1' where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "update devices.ssh_settings set port = $2, auth = 'password' \
         where device_id = $1::text::uuid",
    )
    .bind(PVE_01)
    .bind(i32::from(port))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
         first_seen_at, accepted_at, accepted_by) select $1::text::uuid, 'ssh-ed25519', $2, $3, \
         $4, $4, id from access.operators",
    )
    .bind(PVE_01)
    .bind(PINNED_FINGERPRINT)
    .bind(PINNED)
    .bind(start())
    .execute(pool)
    .await
    .unwrap();
    let device: DeviceId = PVE_01.parse().unwrap();
    let password = SshSecret::Password(Secret::from("stored ssh password".to_owned()));
    let mut conn = pool.acquire().await.unwrap();
    let host = host_of(pool, device).await;
    store_ssh(&mut conn, &test_vault(), (device, &host), password)
        .await
        .unwrap();
}

/// What the chain would change: the device, its SSH settings and its pin.
async fn identity_of_device(pool: &PgPool) -> Vec<String> {
    let mut stored = Vec::new();
    for table in ["devices.devices", "devices.ssh_settings", "trust.host_keys"] {
        stored.extend(rows(pool, table).await);
    }
    stored
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_100_a_write_key_cannot_send_the_stored_password_to_another_host(pool: PgPool) {
    set_operator(&pool).await;
    let device = Listener::start().await;
    let theirs = Listener::start().await;
    trusted_device(&pool, device.port).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let mut parts = dependencies(&pool, clock, Bus::new(16));
    parts.live = Live::default().reaching_loopback_for_tests();
    let hub = Hub::serving(&pool, parts);
    let token = hub.token().await;
    let key = secret(&hub, &token, "ci", "WRITE").await;
    let before = identity_of_device(&pool).await;

    let ssh = json!({ "enabled": true, "port": theirs.port, "username": "operator",
                      "auth": "PASSWORD" });
    let input = json!({ "host": "127.0.0.1", "ssh": ssh });
    let moved = hub
        .post(Some(&key), MOVE, json!({ "id": PVE_01, "input": input }))
        .await;
    assert_eq!(error(&moved), kind_refused(), "updateDevice: {moved}");
    let test = json!({ "input": { "deviceId": PVE_01 } });
    hub.post(Some(&key), TEST, test.clone()).await;
    let pinned = json!({ "id": PVE_01, "fingerprint": THEIRS });
    let accepted = hub.post(Some(&key), ACCEPT, pinned).await;
    assert_eq!(
        error(&accepted),
        kind_refused(),
        "acceptHostKey: {accepted}"
    );
    hub.post(Some(&key), TEST, test).await;

    assert_eq!(identity_of_device(&pool).await, before);
    assert_eq!(
        theirs.accepted(),
        0,
        "the key holder's host was never reached"
    );
    assert_eq!(device.accepted(), 2, "each test went to the stored host");
}
