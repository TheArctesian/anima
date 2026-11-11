//! The warning marks: each warning is marked once for a node, a kind and an expiry
//! (VPN-010, VPN-011, BD-4).

use chrono::Duration;
use sqlx::PgPool;

use super::now;
use crate::{
    events::Severity,
    vpn::{
        store::mark,
        warnings::{Kind, Warning},
    },
};

fn warning(kind: Kind, days: Option<i64>) -> Warning {
    Warning {
        key: format!("nodekey:{:064x}", 6),
        kind,
        key_expiry: days.map(|days| now() + Duration::days(days)),
        device: None,
        severity: Severity::Warning,
        message: "VPN key of laptop expires in 20 days, on 2030-05-25".to_owned(),
    }
}

async fn marked(pool: &PgPool, warning: &Warning) -> bool {
    let mut conn = pool.acquire().await.unwrap();
    mark(&mut conn, warning, now()).await.unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_warning_is_marked_once(pool: PgPool) {
    let expiring = warning(Kind::Expiry30, Some(20));

    assert!(marked(&pool, &expiring).await);
    assert!(!marked(&pool, &expiring).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_renewed_key_is_marked_again(pool: PgPool) {
    assert!(marked(&pool, &warning(Kind::Expiry30, Some(20))).await);

    assert!(marked(&pool, &warning(Kind::Expiry30, Some(110))).await);
    assert!(marked(&pool, &warning(Kind::Expiry14, Some(20))).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_011_not_in_anima_with_no_expiry_is_marked_once(pool: PgPool) {
    let outside = warning(Kind::NotInAnima, None);

    assert!(marked(&pool, &outside).await);
    assert!(!marked(&pool, &outside).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_mark_holds_only_a_node_key(pool: PgPool) {
    let mut odd = warning(Kind::NotInAnima, None);
    odd.key = "PublicKey:0000".to_owned();
    let mut conn = pool.acquire().await.unwrap();

    let refused = mark(&mut conn, &odd, now()).await.unwrap_err();

    let code = refused
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    assert_eq!(code.as_deref(), Some("23514"));
}
