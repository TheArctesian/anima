//! The hosted mode through the hub and a fake link (hosted-mode.md HMD-037, R-129): only the
//! cloud's key makes an invite and a claim, only the device that joined with that invite turns
//! the claim into a token, once, and no one signs in with a password. No invite, claim, token
//! or key reaches an entry or a log line (LOG-007).

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anima_hub::{
    clock::{Clock, FixedClock},
    config,
    fleet::events::{Bus, process_log},
    schema::Dependencies,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::control::{FakeLink, INVITE, KEY, code, ok, written};
use crate::{
    hub::{Hub, dependencies, start},
    logs::Capture,
};

const CLOUD_KEY: &str = "cloud-key-of-the-test-hub-0123456789abcdef";
const DEVICE: &str = "[fd41:4e49:4d41::7]";
const INVITE_CALL: &str = "mutation($k: String!, $n: String!) { \
    hostedDeviceInvite(cloudKey: $k, name: $n) { invite claim expiresAt } }";
const CLAIM_CALL: &str =
    "mutation($c: String!) { claimDeviceToken(claim: $c) { token operator { username } } }";

/// A link that lists the device `Studio` at `DEVICE` once an invite for it was made.
fn hosted_link() -> FakeLink {
    let joined = Arc::new(AtomicBool::new(false));
    FakeLink::start(move |request| match request["verb"].as_str() {
        Some("invites.make") => {
            joined.store(true, Ordering::SeqCst);
            ok(&json!({ "invite": INVITE, "expires": "2030-05-06T12:00:00Z" }))
        }
        _ if joined.load(Ordering::SeqCst) => ok(&json!([{
            "key": KEY, "fingerprint": "1a2b3c4d", "name": "Studio",
            "address": "fd41:4e49:4d41::7", "expires": "2030-08-03T12:00:00Z",
            "streams": 1, "path": "direct",
        }])),
        _ => ok(&json!([])),
    })
}

/// The parts of a hub in the hosted mode over `link`, with the cloud key in a file of `folder`.
fn hosted_parts(pool: &PgPool, link: &FakeLink, folder: &tempfile::TempDir) -> Dependencies {
    let key_file = folder.path().join("cloud_key");
    std::fs::write(&key_file, format!("{CLOUD_KEY}\n")).unwrap();
    let env = BTreeMap::from([
        (
            "ANIMA_LINK_SOCKET".to_owned(),
            "/run/anima/link.sock".to_owned(),
        ),
        (
            "ANIMA_LINK_CONTROL".to_owned(),
            link.path.display().to_string(),
        ),
        (
            "ANIMA_HOSTED_CLOUD_KEY_FILE".to_owned(),
            key_file.display().to_string(),
        ),
    ]);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    Dependencies {
        config: Arc::new(config::from_env(&env).unwrap()),
        ..dependencies(pool, clock, Bus::new(16))
    }
}

/// One hosted hub, seen by the cloud (requests from this computer) and by the device `Studio`
/// (requests from its link address).
fn hubs(pool: &PgPool, link: &FakeLink, folder: &tempfile::TempDir) -> (Hub, Hub) {
    let cloud = Hub::serving(pool, hosted_parts(pool, link, folder));
    let device = cloud.from(DEVICE);
    (cloud, device)
}

async fn invite(cloud: &Hub, key: &str) -> Value {
    cloud
        .post(None, INVITE_CALL, json!({ "k": key, "n": "Studio" }))
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_037_the_cloud_key_makes_an_invite_and_its_device_trades_the_claim_for_a_token(
    pool: PgPool,
) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("debug", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let (link, folder) = (hosted_link(), tempfile::tempdir().unwrap());
    let (cloud, device) = hubs(&pool, &link, &folder);

    let made = invite(&cloud, CLOUD_KEY).await;
    assert!(made["errors"].is_null(), "{made}");
    let made = &made["data"]["hostedDeviceInvite"];
    assert_eq!(made["invite"], INVITE);
    let claim = made["claim"].as_str().unwrap().to_owned();
    assert_eq!(claim.len(), 43, "256 bits in base64url");

    let signed_in = device.post(None, CLAIM_CALL, json!({ "c": claim })).await;
    assert!(signed_in["errors"].is_null(), "{signed_in}");
    let token = signed_in["data"]["claimDeviceToken"]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        signed_in["data"]["claimDeviceToken"]["operator"]["username"],
        "owner"
    );
    let me = device
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["data"]["me"]["username"], "owner", "{me}");

    let again = device.post(None, CLAIM_CALL, json!({ "c": claim })).await;
    assert_eq!(
        code(&again),
        "UNAUTHENTICATED",
        "a claim works once: {again}"
    );
    let stored = written(&pool).await;
    assert!(
        stored.contains("The Anima.ing cloud made a link invite for the device Studio."),
        "{stored}"
    );
    assert!(
        stored.contains("The device Studio signed in with the Anima.ing account."),
        "{stored}"
    );
    for text in [stored, capture.text()] {
        for secret in [INVITE, claim.as_str(), token.as_str(), CLOUD_KEY] {
            assert!(!text.contains(secret), "{text}");
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_129_a_wrong_cloud_key_makes_nothing(pool: PgPool) {
    let (link, folder) = (hosted_link(), tempfile::tempdir().unwrap());
    let (cloud, _) = hubs(&pool, &link, &folder);
    for key in ["", "cloud-key-of-the-test-hub", "x"] {
        assert_eq!(code(&invite(&cloud, key).await), "UNAUTHENTICATED");
    }
    assert_eq!(link.changes(), Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn r_129_a_claim_from_another_device_or_off_the_link_is_refused_and_spent(pool: PgPool) {
    let (link, folder) = (hosted_link(), tempfile::tempdir().unwrap());
    let (cloud, device) = hubs(&pool, &link, &folder);
    let other = cloud.from("[fd41:4e49:4d41::9]");
    let made = invite(&cloud, CLOUD_KEY).await;
    let claim = json!({ "c": made["data"]["hostedDeviceInvite"]["claim"] });

    // From this computer: not the link. The claim is not spent by a sender that is not a device.
    assert_eq!(
        code(&cloud.post(None, CLAIM_CALL, claim.clone()).await),
        "UNAUTHENTICATED"
    );
    // From a different device of the link: refused, and the claim is spent.
    assert_eq!(
        code(&other.post(None, CLAIM_CALL, claim.clone()).await),
        "UNAUTHENTICATED"
    );
    assert_eq!(
        code(&device.post(None, CLAIM_CALL, claim).await),
        "UNAUTHENTICATED"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_037_a_device_already_on_the_link_gets_a_claim_and_no_second_invite(pool: PgPool) {
    let (link, folder) = (hosted_link(), tempfile::tempdir().unwrap());
    let (cloud, device) = hubs(&pool, &link, &folder);
    let first = invite(&cloud, CLOUD_KEY).await;
    let unused = json!({ "c": first["data"]["hostedDeviceInvite"]["claim"] });

    // Studio is on the link now: a second ask gives only a claim, which ends the first one.
    let second = invite(&cloud, CLOUD_KEY).await;
    let second = &second["data"]["hostedDeviceInvite"];
    assert!(second["invite"].is_null(), "{second}");
    assert_eq!(
        code(&device.post(None, CLAIM_CALL, unused).await),
        "UNAUTHENTICATED"
    );
    let claim = json!({ "c": second["claim"] });
    let signed_in = device.post(None, CLAIM_CALL, claim).await;
    assert!(signed_in["errors"].is_null(), "{signed_in}");
    let made: Vec<Value> = link.changes();
    assert_eq!(made.len(), 1, "one invite only: {made:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_129_a_hosted_hub_refuses_every_password_sign_in(pool: PgPool) {
    let (link, folder) = (hosted_link(), tempfile::tempdir().unwrap());
    let (cloud, _) = hubs(&pool, &link, &folder);
    let response = cloud.login("owner", "a password that no one has").await;
    assert_eq!(code(&response), "FORBIDDEN", "{response}");
    assert_eq!(
        response["errors"][0]["message"],
        "This hub is hosted by Anima.ing. Sign in with your Anima.ing account."
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn r_129_a_hub_that_is_not_hosted_has_neither_call(pool: PgPool) {
    let hub = Hub::new(&pool, start());
    assert_eq!(code(&invite(&hub, CLOUD_KEY).await), "FORBIDDEN");
    let claim = hub.post(None, CLAIM_CALL, json!({ "c": "x" })).await;
    assert_eq!(code(&claim), "FORBIDDEN");
}
