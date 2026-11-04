//! A check's parent is a group check of the same device, and groups have no parent, so
//! parents never form a cycle (CHK-004: a group counts as the worst of its children, inside
//! its device's status).

use sqlx::PgPool;
use uuid::Uuid;

use super::{DOCKER_HOST, PVE_01, constraint_of, id, insert_check};

/// Sets `child`'s parent, with `parent_type` as given.
async fn set_parent(
    pool: &PgPool,
    child: Uuid,
    (parent, parent_type): (Uuid, Option<&str>),
) -> Result<(), sqlx::Error> {
    sqlx::query("update checks.checks set parent_id = $1, parent_type = $2 where id = $3")
        .bind(parent)
        .bind(parent_type)
        .bind(child)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn group_of(pool: &PgPool, child: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("select parent_id from checks.checks where id = $1")
        .bind(child)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Inserts checks of the given types and names on `device` and returns their ids.
async fn checks_on<const N: usize>(pool: &PgPool, device: &str, kinds: [&str; N]) -> [Uuid; N] {
    let ids = [(); N].map(|()| Uuid::now_v7());
    for (check, kind) in ids.iter().zip(kinds) {
        let name = format!("{kind} {check}");
        insert_check(pool, (*check, device), (kind, &name), None)
            .await
            .unwrap();
    }
    ids
}

fn refused_by(outcome: Result<(), sqlx::Error>) -> Option<String> {
    constraint_of(&outcome.unwrap_err()).map(ToOwned::to_owned)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_a_parent_must_be_a_group_on_the_same_device(pool: PgPool) {
    let [group, child, sibling] = checks_on(&pool, PVE_01, ["group", "tcp", "icmp"]).await;
    let [elsewhere] = checks_on(&pool, DOCKER_HOST, ["tcp"]).await;
    let group_parent = |parent| (parent, Some("group"));

    let refusals = [
        (
            set_parent(&pool, elsewhere, group_parent(group)).await,
            "checks_parent_fkey",
        ),
        (
            set_parent(&pool, child, group_parent(Uuid::now_v7())).await,
            "checks_parent_fkey",
        ),
        (
            set_parent(&pool, child, group_parent(sibling)).await,
            "checks_parent_fkey",
        ),
        (
            set_parent(&pool, child, (sibling, Some("icmp"))).await,
            "checks_parent_type_check",
        ),
        (
            set_parent(&pool, child, (group, None)).await,
            "checks_parent_type_set_check",
        ),
    ];
    set_parent(&pool, child, group_parent(group)).await.unwrap();

    for (index, (outcome, constraint)) in refusals.into_iter().enumerate() {
        assert_eq!(
            refused_by(outcome).as_deref(),
            Some(constraint),
            "attempt {index}"
        );
    }
    assert_eq!(group_of(&pool, elsewhere).await, None);
    assert_eq!(group_of(&pool, child).await, Some(group));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_groups_have_no_parent_so_parents_cannot_form_a_cycle(pool: PgPool) {
    let [outer, inner, child] = checks_on(&pool, PVE_01, ["group", "group", "tcp"]).await;
    let nested = set_parent(&pool, inner, (outer, Some("group"))).await;
    let itself = set_parent(&pool, outer, (outer, Some("group"))).await;
    // Two checks that name each other in one statement.
    let mutual = |a: Uuid, b: Uuid| {
        sqlx::query(
            "update checks.checks set parent_type = 'group', \
             parent_id = case id when $1 then $2 else $1 end where id in ($1, $2)",
        )
        .bind(a)
        .bind(b)
        .execute(&pool)
    };
    let groups = mutual(outer, inner).await.map(|_| ());
    let mixed = mutual(outer, child).await.map(|_| ());

    for outcome in [nested, itself, groups, mixed] {
        assert_eq!(
            refused_by(outcome).as_deref(),
            Some("checks_group_parent_check")
        );
    }
    for check in [outer, inner, child] {
        assert_eq!(group_of(&pool, check).await, None);
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_a_group_with_children_keeps_its_type_and_device(pool: PgPool) {
    let [group, child] = checks_on(&pool, PVE_01, ["group", "tcp"]).await;
    set_parent(&pool, child, (group, Some("group")))
        .await
        .unwrap();
    let mut refusals = Vec::new();
    for assignment in ["type = 'tcp'", "device_id = $2"] {
        let sql = format!("update checks.checks set {assignment} where id = $1");
        let outcome = sqlx::query(&sql)
            .bind(group)
            .bind(id(DOCKER_HOST))
            .execute(&pool)
            .await;
        refusals.push((assignment, refused_by(outcome.map(|_| ()))));
    }

    for (assignment, constraint) in refusals {
        assert_eq!(
            constraint.as_deref(),
            Some("checks_parent_fkey"),
            "{assignment}"
        );
    }
    assert_eq!(group_of(&pool, child).await, Some(group));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_004_deleting_a_group_keeps_its_children_on_their_device_without_a_parent(
    pool: PgPool,
) {
    let [group, child] = checks_on(&pool, PVE_01, ["group", "tcp"]).await;
    set_parent(&pool, child, (group, Some("group")))
        .await
        .unwrap();

    sqlx::query("delete from checks.checks where id = $1")
        .bind(group)
        .execute(&pool)
        .await
        .unwrap();

    let row: (Uuid, Option<Uuid>, Option<String>) =
        sqlx::query_as("select device_id, parent_id, parent_type from checks.checks where id = $1")
            .bind(child)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row, (id(PVE_01), None, None));
}
