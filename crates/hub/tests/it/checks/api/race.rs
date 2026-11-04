//! Audit entries describe the rows as they were under the mutation's lock, not as they were
//! when the request arrived (LOG-005).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    harness::{PVE_01, UPDATE, create, input, schema_with_bus, with_variables},
    writes::rows,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const DELETE: &str = "mutation($id: ID!) { deleteCheck(id: $id) }";

fn tcp() -> Value {
    json!({ "tcp": { "port": 22 } })
}

fn group_of(ids: &[&str]) -> Value {
    json!({ "group": { "childIds": ids } })
}

/// Long enough for the mutation under test to reach the row another transaction holds.
async fn pause() {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}

fixture_test! {
    async fn log_005_an_update_audits_against_the_row_under_its_lock_not_the_one_it_first_saw(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        // Another transaction holds the row, changes the interval and commits while the
        // rename waits.
        let mut other = pool.begin().await.unwrap();
        sqlx::query("UPDATE checks.checks SET interval_seconds = 120 WHERE id = $1::text::uuid")
            .bind(&id)
            .execute(&mut *other)
            .await
            .unwrap();
        let waiting = {
            let (schema, id) = (schema.clone(), id.clone());
            tokio::spawn(async move {
                let vars = json!({ "id": id, "input": { "name": "renamed" } });
                with_variables(&schema, UPDATE, vars).await
            })
        };
        pause().await;
        other.commit().await.unwrap();
        let response = waiting.await.unwrap();
        assert_eq!(response.errors.len(), 0, "{:?}", response.errors);
        let audits = rows(&pool, "audit_log").await;
        assert_eq!(audits[1]["change"], json!({ "name": "renamed" }));
    }

    async fn log_005_a_group_delete_audits_the_children_it_really_freed(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let kept = create(&schema, input(PVE_01, "kept", tcp())).await;
        let gone = create(&schema, input(PVE_01, "gone", tcp())).await;
        let group = create(&schema, input(PVE_01, "group", group_of(&[&kept, &gone]))).await;
        // Another transaction takes one child out of the group, and commits while the delete waits.
        let mut other = pool.begin().await.unwrap();
        sqlx::query("UPDATE checks.checks SET parent_id = NULL, parent_type = NULL WHERE id = $1::text::uuid")
            .bind(&gone)
            .execute(&mut *other)
            .await
            .unwrap();
        let waiting = {
            let (schema, group) = (schema.clone(), group.clone());
            tokio::spawn(async move { with_variables(&schema, DELETE, json!({ "id": group })).await })
        };
        pause().await;
        other.commit().await.unwrap();
        let response = waiting.await.unwrap();
        assert_eq!(response.errors.len(), 0, "{:?}", response.errors);
        let audits = rows(&pool, "audit_log").await;
        let last = &audits[audits.len() - 1];
        assert_eq!(last["change"]["childIds"], json!([kept]));
    }

    async fn log_005_a_regroup_does_not_take_back_a_child_that_moved_to_another_group(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let a = create(&schema, input(PVE_01, "a", tcp())).await;
        let b = create(&schema, input(PVE_01, "b", tcp())).await;
        let group = create(&schema, input(PVE_01, "group", group_of(&[&a, &b]))).await;
        let other_group = create(&schema, input(PVE_01, "other", group_of(&[]))).await;
        let mut other = pool.begin().await.unwrap();
        sqlx::query("UPDATE checks.checks SET parent_id = $2::text::uuid WHERE id = $1::text::uuid")
            .bind(&b)
            .bind(&other_group)
            .execute(&mut *other)
            .await
            .unwrap();
        let waiting = {
            let (schema, group, a) = (schema.clone(), group.clone(), a.clone());
            tokio::spawn(async move {
                let vars = json!({ "id": group, "input": { "config": group_of(&[&a]) } });
                with_variables(&schema, UPDATE, vars).await
            })
        };
        pause().await;
        other.commit().await.unwrap();
        let response = waiting.await.unwrap();
        assert_eq!(response.errors.len(), 0, "{:?}", response.errors);
        let parent: Option<String> =
            sqlx::query_scalar("SELECT parent_id::text FROM checks.checks WHERE id = $1::text::uuid")
                .bind(&b)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(parent.as_deref(), Some(other_group.as_str()));
    }
}
