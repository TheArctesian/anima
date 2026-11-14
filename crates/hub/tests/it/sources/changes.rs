//! `sourceAccountChanged` sends the account each time a run starts or ends, read as it is when
//! sent (SRC-005, K1).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Actor, Bus, Operator},
    },
    ids::OperatorId,
    schema::{Dependencies, build_schema},
};
use async_graphql::{Request, futures_util::StreamExt as _};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{kit, set_projects};
use crate::hub::{dependencies, start};

#[sqlx::test(migrations = "./migrations")]
async fn src_005_source_account_changed_sends_the_account_when_its_run_starts_and_ends(
    pool: PgPool,
) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1")]]);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        sources: kit.reach.clone(),
        ..dependencies(&pool, clock, Bus::new(16))
    });
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    let query = format!("subscription {{ sourceAccountChanged(id: \"{id}\") {{ id health }} }}");
    let request =
        Identity::for_tests(operator, start() + TimeDelta::days(3650)).attach(Request::new(query));
    let mut stream = schema.execute_stream(request);
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "nothing was published yet");

    kit.run(id).await;
    let mut sent = Vec::new();
    for _ in 0..2 {
        let next = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        assert!(next.errors.is_empty(), "{:?}", next.errors);
        sent.push(next.data.into_json().unwrap()["sourceAccountChanged"].clone());
    }
    assert_eq!(
        sent,
        [
            json!({ "id": id, "health": "OK" }),
            json!({ "id": id, "health": "OK" })
        ]
    );
}
