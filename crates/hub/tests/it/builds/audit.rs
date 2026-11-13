//! Every operator change to builds leaves one audit entry and never the token, and so does each
//! builder that goes with its token (BLD-080, BLD-025, LOG-004, LOG-007).

use anima_hub::fleet::{access, builds};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{claim, create, create_job, data, enrol, operator};
use crate::hub::{CLIENT, Hub, error, set_operator, start};

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
    let job = release["createRelease"]["jobs"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cancel = "mutation($id: ID!) { cancelBuildJob(id: $id) { state cancelledAt } }";
    let cancelled = data(
        hub.post(Some(&operator), cancel, json!({ "id": job }))
            .await,
    );
    assert_eq!(cancelled["cancelBuildJob"]["state"], "CANCELLED");
    let builder = enrol(&hub, &operator, "pop-os").await;
    let revoke = "mutation($id: ID!) { revokeBuilder(id: $id) }";
    data(
        hub.post(Some(&operator), revoke, json!({ "id": builder.id }))
            .await,
    );

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
    let response = hub
        .post(Some(&operator), cancel, json!({ "id": job }))
        .await;
    assert_eq!(error(&response).1, "CONFLICT", "{response}");
    let cancels = audit_rows(&pool)
        .await
        .into_iter()
        .filter(|row| row["action"] == "cancel")
        .count();
    assert_eq!(cancels, 0);
}

/// The operator name, actor kind, token kind and key name of an audit entry.
type Who = (String, Option<String>, Option<String>, Option<String>);

/// The `(action, reason)` of each builder audit entry the hub itself wrote.
async fn removals(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT action, change ->> 'reason' FROM events.audit_log \
         WHERE target_type = 'builder' AND change ? 'reason' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_025_a_builder_pruned_with_its_expired_token_is_audited(pool: PgPool) {
    let operator = operator(&pool).await;
    enrol(&Hub::new(&pool, start()), &operator, "pop-os").await;
    let early = start() + chrono::TimeDelta::days(179);
    let before = builds::builders(&pool).await.unwrap();
    access::prune(&pool, early).await.unwrap();
    let gone = builds::audit_removed(&pool, &before, early, builds::Removal::TokenExpired);
    assert_eq!(gone.await.unwrap(), 0);
    let late = start() + chrono::TimeDelta::days(180);
    let before = builds::builders(&pool).await.unwrap();
    access::prune(&pool, late).await.unwrap();
    let gone = builds::audit_removed(&pool, &before, late, builds::Removal::TokenExpired);
    assert_eq!(gone.await.unwrap(), 1);
    assert_eq!(
        builds::builders(&pool).await.unwrap(),
        [] as [builds::Builder; 0]
    );
    let expected = vec![("delete".to_owned(), "token expired".to_owned())];
    assert_eq!(removals(&pool).await, expected);
    // The prune is the hub's own work: Hub, with no token and no key (LOG-014, LOG-018, OD-15).
    let who: Vec<Who> = sqlx::query_as(
        "SELECT operator_name, actor_kind, token_kind, api_key_name FROM events.audit_log \
         WHERE target_type = 'builder' AND change ? 'reason'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        who,
        [("Anima".to_owned(), Some("hub".to_owned()), None, None)]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_025_builders_gone_with_changed_credentials_are_audited(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    enrol(&hub, &operator, "pop-os").await;
    enrol(&hub, &operator, "mac").await;
    let before = builds::builders(&pool).await.unwrap();
    set_operator(&pool).await;
    let why = builds::Removal::CredentialsChanged;
    let gone = builds::audit_removed(&pool, &before, start(), why).await;
    assert_eq!(gone.unwrap(), 2);
    let reason = ("delete".to_owned(), "credentials changed".to_owned());
    assert_eq!(removals(&pool).await, vec![reason.clone(), reason]);
}
