//! Every operator change to builds leaves one audit entry and never the token (BLD-080,
//! LOG-004, LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{claim, create, create_job, data, enrol, operator};
use crate::hub::{CLIENT, Hub, error, start};

async fn audit_rows(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM events.audit_log t WHERE target_type <> 'operator' \
         ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_080_create_cancel_enrol_and_revoke_are_audited(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let release = data(create(&hub, &operator, "0.1.12").await);
    let release_id = release["createRelease"]["id"].as_str().unwrap().to_owned();
    let job = release["createRelease"]["jobs"][0]["id"].as_str().unwrap().to_owned();
    let cancel = "mutation($id: ID!) { cancelBuildJob(id: $id) { state cancelledAt } }";
    let cancelled = data(hub.post(Some(&operator), cancel, json!({ "id": job })).await);
    assert_eq!(cancelled["cancelBuildJob"]["state"], "CANCELLED");
    let builder = enrol(&hub, &operator, "pop-os").await;
    let revoke = "mutation($id: ID!) { revokeBuilder(id: $id) }";
    data(hub.post(Some(&operator), revoke, json!({ "id": builder.id })).await);

    let rows = audit_rows(&pool).await;
    let summary: Vec<(&str, &str, &str)> = rows
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap();
            (text("action"), text("target_type"), text("target_id"))
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("create", "release", release_id.as_str()),
            ("cancel", "build_job", job.as_str()),
            ("create", "builder", builder.id.as_str()),
            ("delete", "builder", builder.id.as_str()),
        ]
    );
    for row in &rows {
        assert_eq!(row["client_addr"], CLIENT);
        assert!(!row.to_string().contains(&builder.token), "{row}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_008_only_a_queued_job_can_be_cancelled(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    let cancel = "mutation($id: ID!) { cancelBuildJob(id: $id) { state } }";
    let response = hub.post(Some(&operator), cancel, json!({ "id": job })).await;
    assert_eq!(error(&response).1, "CONFLICT", "{response}");
    let cancels = audit_rows(&pool)
        .await
        .into_iter()
        .filter(|row| row["action"] == "cancel")
        .count();
    assert_eq!(cancels, 0);
}
