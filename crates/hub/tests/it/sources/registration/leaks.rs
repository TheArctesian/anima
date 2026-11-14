//! LOG-007, PRJ-018, PRJ-041: after the whole flow, the App's key, GitHub's code, the ticket,
//! both states and the cookie value are in no process log line (at trace), no table (audit and
//! events included), no GraphQL answer and no `githubConnectChanged` payload. GitHub's client
//! and webhook secrets are nowhere either.

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Actor, Bus, ClientAddr, Operator, process_log},
    },
    ids::OperatorId,
    schema::{Dependencies, build_schema},
};
use async_graphql::{
    Request,
    futures_util::{StreamExt as _, stream::BoxStream},
};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{
    STATE,
    browser::{Browser, CODE, INSTALLATION, PUBLIC, Served, install_state, served},
    link, started,
};
use crate::{
    hub::{dependencies, start},
    logs::Capture,
    sources::{TEST_KEY, leaks::every_row, set_projects},
};

/// `githubConnectChanged` on a schema that shares the served hub's sources.
fn changes(served: &Served) -> BoxStream<'static, async_graphql::Response> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        sources: served.kit.reach.clone(),
        ..dependencies(&served.kit.pool, clock, Bus::new(16))
    });
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    let query = "subscription { githubConnectChanged { id phase startedBy pageUrl appId appSlug \
        ownerLogin installUrl appSettingsUrl failure message account { id label instanceKey } } }";
    let request = Identity::for_tests(operator, start() + TimeDelta::days(3650))
        .attach(Request::new(query))
        .data(ClientAddr("100.64.0.7".parse().unwrap()));
    Box::pin(schema.execute_stream(request))
}

async fn answer(served: &Served) -> String {
    served
        .kit
        .hub
        .post_text(Some(&served.kit.token), STATE, json!({}))
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_prj_041_no_value_of_the_flow_is_logged_stored_or_answered(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let served = served(&pool, Duration::from_secs(3600)).await;
    set_projects(&pool, true).await;
    served.kit.stub.set_pages(&[&[("a", "1")]]);
    let mut stream = changes(&served);
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "subscribed; nothing was published yet");
    let mut answers = Vec::new();
    let id = started(&served.kit).await;
    let link = link(&served.kit, &id).await;
    let ticket = link.split("ticket=").nth(1).unwrap().to_owned();
    let mut browser = Browser::new(&served);
    let s1 = browser.open(&link).await;
    let cookie = browser.cookies["__Host-anima-connect"].clone();
    answers.push(answer(&served).await);
    let s2 = install_state(&browser.created(&s1).await);
    answers.push(answer(&served).await);
    let done = browser.installed(INSTALLATION, &s2).await;
    assert!(done.body.contains("GitHub is connected."), "{}", done.body);
    answers.push(answer(&served).await);
    let mut sent = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(500), stream.next()).await
    {
        assert!(next.errors.is_empty(), "{:?}", next.errors);
        sent.push(next.data.into_json().unwrap().to_string());
    }
    assert!(sent.len() >= 4, "each phase was sent: {sent:?}");
    assert!(sent.last().unwrap().contains("CONNECTED"));

    let rows = every_row(&pool).await;
    let log = capture.text();
    assert!(
        log.contains("/connect/github/created"),
        "the requests were logged"
    );
    assert!(
        rows.contains("sources.github.connect"),
        "the audit entries were read"
    );
    assert!(rows.contains("anima-k3f9x2mq7b"), "App created was audited");
    let pem_line = TEST_KEY.lines().nth(3).unwrap().to_owned();
    let needles = [
        ("the key", pem_line.as_str()),
        ("a key", "PRIVATE KEY"),
        ("the code", CODE),
        ("the ticket", ticket.as_str()),
        ("S1", s1.as_str()),
        ("S2", s2.as_str()),
        ("the cookie", cookie.as_str()),
        ("the client secret", "CLIENT-SECRET-MARKER-0000"),
        ("the webhook secret", "WEBHOOK-SECRET-MARKER-1111"),
    ];
    for (place, text) in [("a table", &rows), ("the log", &log)]
        .into_iter()
        .chain(answers.iter().map(|a| ("an answer", a)))
        .chain(sent.iter().map(|p| ("a githubConnectChanged payload", p)))
    {
        for (what, needle) in needles {
            assert!(!text.contains(needle), "{what} is in {place}");
        }
    }
    assert!(link.starts_with(PUBLIC));
}
