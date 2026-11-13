//! A push under maintenance is recorded, published and shown as Maintenance, the same as a
//! scheduled attempt (CHK-006).

use anima_hub::{fleet::events::Signal, status::Status};
use async_graphql::futures_util::StreamExt as _;
use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use super::super::harness::{data_with, signed_in};
use super::rig;

/// `jellyfin` in the K6 fixtures: the manual maintenance flag is set.
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";

const READ: &str = "query($id: ID!) { check(id: $id) { status \
    heartbeats(first: 1) { edges { node { status } } } } }";

macro_rules! push_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

push_test! {
    async fn chk_006_a_pushed_down_under_manual_maintenance_is_maintenance_everywhere(pool: PgPool) {
        let rig = rig(pool).await;
        let (id, path) = rig.push_check_on(JELLYFIN, "backup", 0).await;
        let mut receiver = rig.bus.subscribe();
        let query = format!("subscription {{ heartbeatAdded(checkId: \"{id}\") {{ heartbeat {{ status }} }} }}");
        let mut stream = Box::pin(rig.schema.execute_stream(signed_in(&query)));
        // The stream subscribes when first polled; nothing is published yet, so it waits.
        let pending = tokio::time::timeout(std::time::Duration::from_millis(50), stream.next()).await;
        assert!(pending.is_err(), "the subscription answered before the push");

        assert_eq!(rig.get(&format!("{path}?status=down&msg=disk%20full")).await.0, StatusCode::OK);

        assert_eq!(rig.beats(&id).await[0].0, "maintenance");
        match receiver.try_recv() {
            Ok(Signal::Heartbeat { beat, .. }) => assert_eq!(beat.status, Status::Maintenance),
            other => panic!("expected a heartbeat signal, got {other:?}"),
        }
        let live = stream.next().await.expect("the stream ended");
        assert_eq!(live.errors.len(), 0, "{:?}", live.errors);
        let live = live.data.into_json().unwrap();
        assert_eq!(live["heartbeatAdded"]["heartbeat"]["status"], "MAINTENANCE");
        let read = data_with(&rig.schema, READ, json!({ "id": id })).await;
        assert_eq!(read["check"]["heartbeats"]["edges"][0]["node"]["status"], "MAINTENANCE");
        assert_eq!(read["check"]["status"], "MAINTENANCE");
    }
}
