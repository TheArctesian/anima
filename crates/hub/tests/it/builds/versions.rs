//! A version whose builds all failed or were cancelled may be released again; one under way or
//! published may not (BLD-002, D-7).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{claim, create, create_job, data, enrol, operator};
use crate::hub::{Hub, error, start};

const CANCEL: &str = "mutation($id: ID!) { cancelBuildJob(id: $id) { state } }";
const FAIL: &str = "mutation($id: ID!) { reportBuildJob(id: $id, state: FAILED, logLines: [], \
                    failureReason: \"gates failed\") { state } }";

#[sqlx::test(migrations = "./migrations")]
async fn d_7_a_cancelled_or_failed_version_can_be_released_again(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let cancelled = create_job(&hub, &operator, "0.1.12").await;
    data(
        hub.post(Some(&operator), CANCEL, json!({ "id": cancelled }))
            .await,
    );
    let failed = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    let report = data(
        hub.post(Some(&builder.token), FAIL, json!({ "id": failed }))
            .await,
    );
    assert_eq!(report["reportBuildJob"]["state"], "FAILED");

    data(create(&hub, &operator, "0.1.12").await);
    let under_way = create(&hub, &operator, "0.1.12").await;
    assert_eq!(error(&under_way).1, "VALIDATION", "{under_way}");
    let releases: i64 = sqlx::query_scalar("SELECT count(*) FROM builds.releases")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(releases, 3);
}
