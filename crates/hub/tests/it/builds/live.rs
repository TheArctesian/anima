//! `buildJobUpdated` delivers each change of the job asked for, with the log lines it added,
//! and nothing of other jobs (BLD-081).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        builds::{self, BuildQueue, JobState, Report, Updates},
        events::{Bus, Operator},
    },
    ids::{BuildJobId, OperatorId},
    schema::{Dependencies, build_schema},
};
use async_graphql::{Request, Response, Variables, futures_util::StreamExt as _};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{create_job, enrol, operator};
use crate::hub::{Hub, dependencies, start};

const SUBSCRIPTION: &str = "subscription($jobId: ID) { buildJobUpdated(jobId: $jobId) { \
    job { id state builderName } logLines { sequence text } } }";

fn signed_in(job: &str) -> Request {
    let operator = Operator {
        id: OperatorId::from(uuid::Uuid::now_v7()),
        name: "operator".to_owned(),
        actor: anima_hub::fleet::events::Actor::for_tests(),
    };
    let request =
        Request::new(SUBSCRIPTION).variables(Variables::from_json(json!({ "jobId": job })));
    Identity::for_tests(operator, start() + TimeDelta::days(1)).attach(request)
}

fn update(response: Option<Response>) -> Value {
    let response = response.expect("the subscription ended");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["buildJobUpdated"].clone()
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_081_a_subscriber_gets_the_changes_and_lines_of_its_job_only(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let other = create_job(&hub, &operator, "0.1.1").await;
    let watched = create_job(&hub, &operator, "0.1.2").await;
    enrol(&hub, &operator, "first").await;
    enrol(&hub, &operator, "second").await;
    let enrolled = builds::builders(&pool).await.unwrap();

    let (bus, updates) = (Bus::new(4), Updates::new());
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        build_updates: updates.clone(),
        ..dependencies(&pool, clock, bus.clone())
    });
    let mut stream = schema.execute_stream(signed_in(&watched));
    // The first poll runs the resolver, which subscribes; nothing is published yet.
    let early = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(early.is_err(), "an update came before any change");

    let queue = BuildQueue {
        db: &pool,
        bus: &bus,
        updates: &updates,
    };
    let first = builds::claim(queue, &enrolled[0], start()).await.unwrap();
    assert_eq!(first.unwrap().id.to_string(), other);
    builds::claim(queue, &enrolled[1], start()).await.unwrap();
    let report = Report {
        job: watched.parse::<BuildJobId>().unwrap(),
        state: Some(JobState::Building),
        reason: None,
        lines: vec!["Compiling anima".to_owned()],
    };
    builds::report(queue, &enrolled[1], report, start())
        .await
        .unwrap();

    let claimed = update(stream.next().await);
    assert_eq!(claimed["job"]["id"], watched.as_str());
    assert_eq!(claimed["job"]["state"], "CLAIMED");
    assert_eq!(claimed["job"]["builderName"], "second");
    assert_eq!(claimed["logLines"], json!([]));
    let building = update(stream.next().await);
    assert_eq!(building["job"]["state"], "BUILDING");
    assert_eq!(
        building["logLines"],
        json!([{ "sequence": 1, "text": "Compiling anima" }])
    );
}
