//! `access.permitted_networks` and `access::permit_networks` (K2, NET-001, D-10).

use sqlx::PgPool;
use uuid::Uuid;

use super::permit;

const CONFIGURED: &str = "ANIMA_PERMITTED_NETWORKS";

/// `(cidr, label)` of every permitted network, in address order.
async fn stored(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as("select cidr::text, label from access.permitted_networks order by cidr")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn id_of(pool: &PgPool, cidr: &str) -> Uuid {
    sqlx::query_scalar("select id from access.permitted_networks where cidr = $1::cidr")
        .bind(cidr)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn row(cidr: &str, label: &str) -> (String, String) {
    (cidr.to_owned(), label.to_owned())
}

#[sqlx::test(migrations = "./migrations")]
async fn net_001_permit_networks_makes_the_configured_rows_match_the_variable(pool: PgPool) {
    sqlx::query(
        "insert into access.permitted_networks (id, cidr, label) values \
         ('0190a000-0000-7000-8000-0000000000a1', '192.168.50.0/24', 'site lab'), \
         ('0190a000-0000-7000-8000-0000000000a2', '172.16.0.0/12', 'site office')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let given = [
        "10.1.2.3/8",
        "::ffff:100.64.0.0/106",
        "100.64.0.0/10",
        "fd00::1/8",
        "172.16.0.0/12",
    ];
    permit(&pool, &given).await;
    assert_eq!(
        stored(&pool).await,
        vec![
            row("10.0.0.0/8", CONFIGURED),
            row("100.64.0.0/10", CONFIGURED),
            row("172.16.0.0/12", "site office"),
            row("192.168.50.0/24", "site lab"),
            row("fd00::/8", CONFIGURED),
        ]
    );
    let kept = id_of(&pool, "100.64.0.0/10").await;
    permit(&pool, &["100.64.0.0/10"]).await;
    assert_eq!(
        stored(&pool).await,
        vec![
            row("100.64.0.0/10", CONFIGURED),
            row("172.16.0.0/12", "site office"),
            row("192.168.50.0/24", "site lab"),
        ],
        "a network taken out of the variable is no longer permitted"
    );
    assert_eq!(
        id_of(&pool, "100.64.0.0/10").await,
        kept,
        "a kept row stays"
    );
    permit(&pool, &[]).await;
    assert_eq!(stored(&pool).await.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn k2_the_table_refuses_host_bits_and_duplicate_networks(pool: PgPool) {
    let insert = |id: &'static str, cidr: &'static str| {
        sqlx::query(
            "insert into access.permitted_networks (id, cidr, label) \
             values ($1::uuid, $2::cidr, 'x')",
        )
        .bind(id)
        .bind(cidr)
        .execute(&pool)
    };
    let first = insert("0190a000-0000-7000-8000-0000000000b1", "10.0.0.0/8").await;
    assert!(first.is_ok(), "{first:?}");
    let duplicate = insert("0190a000-0000-7000-8000-0000000000b2", "10.0.0.0/8").await;
    assert!(duplicate.is_err());
    let host_bits = insert("0190a000-0000-7000-8000-0000000000b3", "10.0.0.1/8").await;
    assert!(host_bits.is_err());
}
