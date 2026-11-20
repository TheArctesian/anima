//! HMD-069, HMD-070: the attempt on a hub in the link mode, with and without a public origin.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::events::Bus,
    schema::Dependencies,
};
use serde_json::json;
use sqlx::PgPool;

use super::{
    CANCEL, OPEN, STATE,
    api::{PAGE, audited},
    started,
};
use crate::{
    hub::{dependencies, error, start},
    sources::{Kit, kit},
};

/// The same hub in the link mode, as a hub of the pool or of this computer runs: with the
/// public origin that `given` names, or with none.
fn linked(kit: &Kit, given: Option<&str>) -> crate::hub::Hub {
    let mut env = std::collections::BTreeMap::from([(
        "ANIMA_LINK_SOCKET".to_owned(),
        "/run/anima/link.sock".to_owned(),
    )]);
    if let Some(origin) = given {
        env.insert("ANIMA_PUBLIC_ORIGIN".to_owned(), origin.to_owned());
    }
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let parts = Dependencies {
        config: Arc::new(anima_hub::config::from_env(&env).unwrap()),
        sources: kit.reach.clone(),
        ..dependencies(&kit.pool, clock, Bus::new(16))
    };
    crate::hub::Hub::serving(&kit.pool, parts)
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_070_a_hub_that_no_browser_can_open_starts_no_attempt_and_makes_no_ticket(
    pool: PgPool,
) {
    let kit = kit(&pool).await;
    let hub = linked(&kit, None);
    let token = Some(kit.token.as_str());
    let refused = hub
        .post(token, super::START, json!({ "input": null }))
        .await;
    assert_eq!(error(&refused).1, "CONFLICT", "{refused}");
    assert!(
        error(&refused)
            .0
            .contains("no address that a browser can open"),
        "{refused}"
    );
    let state = hub.post(token, STATE, json!({})).await;
    assert!(
        state["data"]["githubConnect"].is_null(),
        "no attempt: {state}"
    );
    assert!(audited(&pool).await.is_empty(), "nothing was started");

    // An attempt of before the rule: no link for it, and it can still be cancelled.
    let id = started(&kit).await;
    let no_link = hub.post(token, OPEN, json!({ "id": id })).await;
    assert_eq!(error(&no_link).1, "CONFLICT", "{no_link}");
    let cancelled = hub.post(token, CANCEL, json!({ "id": id })).await;
    assert!(cancelled["errors"].is_null(), "{cancelled}");
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_069_a_hub_that_was_given_its_public_origin_builds_the_link_from_it(pool: PgPool) {
    let kit = kit(&pool).await;
    let hub = linked(&kit, Some("https://hub.anima.ing"));
    let token = Some(kit.token.as_str());
    let made = hub
        .post(token, super::START, json!({ "input": null }))
        .await;
    assert!(made["errors"].is_null(), "{made}");
    assert_eq!(made["data"]["startGithubConnect"]["pageUrl"], PAGE);
    let id = made["data"]["startGithubConnect"]["id"].as_str().unwrap();
    let opened = hub.post(token, OPEN, json!({ "id": id })).await;
    let url = opened["data"]["openGithubConnect"]["url"].as_str().unwrap();
    assert!(url.starts_with("https://hub.anima.ing/connect/github?ticket="));
}
