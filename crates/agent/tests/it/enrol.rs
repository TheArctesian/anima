//! `anima-agent enrol` against the fake hub (MET-012, K8 Section 3).

use agent_core::wire::EnrolmentCode;
use anima_agent::config::home::Home;
use anima_agent::config::origin::Origin;
use anima_agent::hub::HubClient;
use anima_agent::hub::enrolment::{EnrolError, Enrolment, REFUSED};
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
    let home = Home::at(dir.path().to_path_buf());
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone()).expect("a client");
    let origin = Origin::parse("https://hub.example").expect("an origin");
    let name = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
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
    let home = Home::at(dir.path().to_path_buf());
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone()).expect("a client");
    let origin = Origin::default_hub();
    let error = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
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
    let home = Home::at(dir.path().to_path_buf());
    let client = HubClient::new(hub.url.clone()).expect("a client");
    let store = TokenStore::open(&home);
    let origin = Origin::default_hub();
    let error = Enrolment {
        hub: &client,
        origin,
        home: &home,
        store: &store,
    }
    .spend(code())
    .await
    .expect_err("fails");
    assert!(matches!(error, EnrolError::Failed(_)));
    assert!(error.to_string().contains("make a new one"));
}
