//! No share token reaches the process log (LOG-007), not even when the keeper cannot write a
//! share's end: then it logs the share's id, and the share is removed all the same (S-5).

use std::time::Duration;

use sqlx::PgPool;

use super::{Bench, bench, schema, share_audits, share_end};
use crate::{
    events::process_log,
    sessions::tests::fixture::{Capture, operator, own_pool},
};

/// The token of a share link: what follows `#`.
fn token_of(link: &serde_json::Value) -> String {
    let url = link["url"].as_str().unwrap();
    url.split_once('#').unwrap().1.to_owned()
}

fn id_of(link: &serde_json::Value) -> String {
    link["share"]["id"].as_str().unwrap().to_owned()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_no_share_token_is_logged_even_when_a_share_end_cannot_be_written(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in the tunnels' tests: a second registered dispatcher makes every callsite
    // registration ask all of them, so another test thread cannot disable these callsites.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    let started = bench(&pool).await;
    let own = own_pool(&pool).await;
    let bench = Bench {
        schema: schema(&own, &started.live),
        ..started
    };
    let first = bench.shared().await;
    let ended = bench
        .end_share_as(operator(), "100.64.0.9", &id_of(&first))
        .await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    share_end(&pool, &id_of(&first)).await;

    let second = bench.shared().await;
    own.close().await;
    let ended = bench
        .end_share_as(operator(), "100.64.0.9", &id_of(&second))
        .await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);

    let read = || String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    for _ in 0..250 {
        if read().contains("could not be recorded") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let log = read();
    assert!(
        log.contains("log_007 capture marker"),
        "the capture works: {log}"
    );
    assert!(
        log.contains("the end of a session share could not be recorded")
            && log.contains(&id_of(&second)),
        "the failed write is logged with the share's id: {log}"
    );
    assert!(
        bench.live.shares().hashes().is_empty(),
        "removed all the same"
    );
    let audits = share_audits(&pool).await;
    assert_eq!(audits.len(), 3, "two creates and one end: {audits:?}");
    for secret in [token_of(&first), token_of(&second), "/shared#".to_owned()] {
        assert!(!log.contains(&secret), "{secret} in the log");
    }
}
