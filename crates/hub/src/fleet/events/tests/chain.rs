//! The hash chain of the audit log against PostgreSQL: each write links its entry, the start
//! links the entries that exist, and the check finds each kind of change at its place
//! (LOG-021 to LOG-028).

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{day, new_audit};
use crate::fleet::events::{
    Bus, Source, audit,
    chain::{ChainCheck, FaultKind, check_chain, check_chain_daily, link_existing},
};

/// A time inside the retention of every test entry.
fn soon() -> DateTime<Utc> {
    day(2031, 7, 1)
}

/// Writes `count` audit entries through the hub, each in its own transaction, and returns
/// their IDs in the order of writing.
async fn write(pool: &PgPool, at: DateTime<Utc>, count: usize) -> Vec<Uuid> {
    let mut ids = Vec::new();
    for n in 0..count {
        let mut tx = pool.begin().await.unwrap();
        audit(
            &mut tx,
            at,
            new_audit(json!({ "name": format!("router-{n}") })),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        ids.push(newest(pool).await);
    }
    ids
}

async fn newest(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("select entry_id from events.audit_chain order by seq desc limit 1")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn execute(conn: &mut PgConnection, sql: &str, id: Uuid) {
    sqlx::query(sql).bind(id).execute(conn).await.unwrap();
}

/// The kind and link of the first fault the check finds at `now`.
async fn fault(pool: &PgPool, now: DateTime<Utc>) -> (FaultKind, Option<i64>, Uuid) {
    let fault = check_chain(pool, now)
        .await
        .unwrap()
        .fault
        .expect("a fault");
    (fault.kind, fault.seq, fault.entry_id)
}

async fn whole(pool: &PgPool, now: DateTime<Utc>) -> ChainCheck {
    let check = check_chain(pool, now).await.unwrap();
    assert_eq!(check.fault, None);
    check
}

#[sqlx::test(migrations = "./migrations")]
async fn log_021_a_chain_of_several_entries_written_by_the_hub_is_whole(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 5).await;

    let check = whole(&pool, soon()).await;

    assert_eq!((check.links, check.dropped), (5, 0));
    assert_eq!(check.head.map(|(seq, _)| seq), Some(5));
    assert_eq!(newest(&pool).await, ids[4]);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_027_a_changed_field_is_found_at_its_link(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 4).await;
    let mut conn = pool.acquire().await.unwrap();

    execute(
        &mut conn,
        "update events.audit_log set change = '{\"name\": \"other\"}' where id = $1",
        ids[2],
    )
    .await;

    assert_eq!(
        fault(&pool, soon()).await,
        (FaultKind::Changed, Some(3), ids[2])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_027_entries_that_changed_places_are_found_at_the_first_of_them(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 4).await;
    let mut conn = pool.acquire().await.unwrap();

    sqlx::query(
        "update events.audit_log a set change = b.change from events.audit_log b \
         where (a.id, b.id) in (($1, $2), ($2, $1))",
    )
    .bind(ids[1])
    .bind(ids[2])
    .execute(&mut *conn)
    .await
    .unwrap();

    assert_eq!(
        fault(&pool, soon()).await,
        (FaultKind::Changed, Some(2), ids[1])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_027_a_removed_entry_is_found_at_its_link(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 4).await;
    let mut conn = pool.acquire().await.unwrap();

    execute(
        &mut conn,
        "delete from events.audit_log where id = $1",
        ids[1],
    )
    .await;

    assert_eq!(
        fault(&pool, soon()).await,
        (FaultKind::Missing, Some(2), ids[1])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_027_a_removed_link_breaks_the_chain_at_the_next_link(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 4).await;
    let mut conn = pool.acquire().await.unwrap();

    execute(
        &mut conn,
        "delete from events.audit_chain where entry_id = $1",
        ids[1],
    )
    .await;

    assert_eq!(
        fault(&pool, soon()).await,
        (FaultKind::Broken, Some(3), ids[2])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_027_an_inserted_entry_has_no_link(pool: PgPool) {
    write(&pool, day(2031, 6, 30), 2).await;
    let mut conn = pool.acquire().await.unwrap();
    let forged = Uuid::now_v7();

    execute(
        &mut conn,
        "insert into events.audit_log \
         (id, at, operator_id, operator_name, action, target_type, target_id, client_addr, change) \
         values ($1, '2031-06-30T08:00:00Z', gen_random_uuid(), 'operator', 'delete', \
         'device', 'lab', '100.64.0.7', '{}')",
        forged,
    )
    .await;

    assert_eq!(
        fault(&pool, soon()).await,
        (FaultKind::NotLinked, None, forged)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_024_two_writers_at_the_same_time_make_one_whole_chain(pool: PgPool) {
    let writers: Vec<_> = (0..2)
        .map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move { write(&pool, day(2031, 6, 30), 10).await })
        })
        .collect();
    for writer in writers {
        writer.await.unwrap();
    }

    let check = whole(&pool, soon()).await;

    assert_eq!(check.links, 20);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_026_the_entries_that_exist_are_linked_once_and_the_chain_continues(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    for (n, at) in [
        "2031-06-29T08:00:00.000001Z",
        "2031-06-28T08:00:00Z",
        "2031-06-30T08:00:00Z",
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query("select events.ensure_day_partition('audit_log', $1::timestamptz)")
            .bind(at)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(
            "insert into events.audit_log (id, at, operator_id, operator_name, action, \
             target_type, target_id, client_addr, change) \
             values (gen_random_uuid(), $1::timestamptz, gen_random_uuid(), 'operator', \
             'update', 'device', 'lab', '2001:db8::7', $2)",
        )
        .bind(at)
        .bind(json!({ "n": n, "ratio": 0.1, "big": 1e21 }))
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    assert_eq!(link_existing(&pool).await.unwrap(), 3);
    assert_eq!(link_existing(&pool).await.unwrap(), 0);
    write(&pool, day(2031, 6, 30), 1).await;

    let check = whole(&pool, soon()).await;
    assert_eq!(check.links, 4);
    let oldest: DateTime<Utc> =
        sqlx::query_scalar("select entry_at from events.audit_chain where seq = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        oldest,
        "2031-06-28T08:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_025_a_chain_across_a_dropped_day_is_whole_after_the_retention(pool: PgPool) {
    write(&pool, day(2031, 6, 1), 2).await;
    write(&pool, day(2031, 6, 30), 2).await;
    let mut conn = pool.acquire().await.unwrap();

    sqlx::query("drop table events.audit_log_20310601")
        .execute(&mut *conn)
        .await
        .unwrap();

    let after_a_year = day(2032, 6, 20);
    let check = whole(&pool, after_a_year).await;
    assert_eq!((check.links, check.dropped), (4, 2));
    let (kind, seq, _) = fault(&pool, soon()).await;
    assert_eq!((kind, seq), (FaultKind::Missing, Some(1)));
}

#[sqlx::test(migrations = "./migrations")]
async fn log_028_the_daily_check_writes_an_error_event_only_for_a_fault(pool: PgPool) {
    let ids = write(&pool, day(2031, 6, 30), 2).await;
    let bus = Bus::new(4);
    let errors = async || -> Vec<(String, String)> {
        sqlx::query_as(
            "select source, message from events.event_log where severity = 'error' \
             and actor_kind = 'hub'",
        )
        .fetch_all(&pool)
        .await
        .unwrap()
    };

    check_chain_daily(&pool, &bus, soon()).await.unwrap();
    assert_eq!(errors().await, Vec::new());

    let mut conn = pool.acquire().await.unwrap();
    execute(
        &mut conn,
        "delete from events.audit_log where id = $1",
        ids[0],
    )
    .await;
    check_chain_daily(&pool, &bus, soon() + TimeDelta::days(1))
        .await
        .unwrap();

    assert_eq!(
        errors().await,
        [(
            Source::Hub.as_str().to_owned(),
            "The audit log chain is broken at link 1: an entry was removed.".to_owned()
        )]
    );
}
