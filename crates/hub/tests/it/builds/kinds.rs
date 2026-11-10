//! A builder token is its own token kind with the builder scope (R-107, D-1, D-23): it travels
//! only as a plain bearer, stands for no operator on the tunnels, and the database keeps the kind
//! and the scope together (BLD-022).

use anima_hub::access;
use sqlx::PgPool;

use super::harness::{claim, create_job, data, enrol, operator};
use crate::hub::{Hub, error, start, unauthenticated};

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_builder_token_is_stored_as_the_builder_kind_and_scope(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    enrol(&hub, &operator, "pop-os").await;
    let stored: (String, String) = sqlx::query_as(
        "SELECT t.kind, t.scope FROM access.api_tokens t \
         JOIN builds.builders b ON b.token_id = t.id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, ("builder".to_owned(), "builder".to_owned()));
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_builder_token_opens_no_tunnel(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    let tunnel = access::operator_for_token(&pool, &builder.token, start()).await;
    assert!(matches!(tunnel, Err(access::Error::Unauthenticated)));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_builder_token_spelled_as_an_api_key_is_no_identity(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    let as_key = format!("anima_{}", builder.token);
    assert_eq!(error(&claim(&hub, &as_key).await), unauthenticated());
    let claimed = data(claim(&hub, &builder.token).await);
    assert_eq!(claimed["claimBuildJob"]["state"], "CLAIMED");
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_the_database_keeps_the_builder_scope_on_the_builder_kind_alone(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    enrol(&hub, &operator, "pop-os").await;
    let token: uuid::Uuid = sqlx::query_scalar("SELECT token_id FROM builds.builders")
        .fetch_one(&pool)
        .await
        .unwrap();
    for (kind, scope) in [
        ("builder", "write"),
        ("builder", "read"),
        ("login", "builder"),
    ] {
        let changed =
            sqlx::query("UPDATE access.api_tokens SET kind = $2, scope = $3 WHERE id = $1")
                .bind(token)
                .bind(kind)
                .bind(scope)
                .execute(&pool)
                .await;
        assert!(changed.is_err(), "{kind} {scope}");
    }
}
