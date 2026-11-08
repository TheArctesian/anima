//! K2 "Roles", "Partitions": the runtime role writes and reads notifications and manages
//! partitions only through the owner's functions.

use chrono::TimeDelta;
use sqlx::PgPool;
use uuid::Uuid;

use super::{PING, check, fields, now, refusal, runtime_pool};
use crate::{
    ids::ChannelId,
    notifications::{
        CheckSettings, CheckState, Delivery, NewOutbox, NotificationKind, OutboxId,
        create_partitions_ahead, delete_finished_outbox, store as sql,
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn db_009_runtime_role_does_everything_the_hub_needs(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let at = now(&pool).await;
    let id = ChannelId::from(Uuid::now_v7());
    let mut conn = runtime.acquire().await.unwrap();

    sql::insert_channel(&mut conn, id, &fields("ops"), at)
        .await
        .unwrap();
    let mut changed = fields("ops");
    changed.on_by_default = true;
    assert!(
        sql::update_channel(&mut conn, id, &changed, at)
            .await
            .unwrap()
    );
    sql::set_check_channels(&mut conn, check(PING), &[id])
        .await
        .unwrap();
    let settings = CheckSettings {
        resend_every_beats: 3,
    };
    sql::write_check_settings(&mut conn, check(PING), settings)
        .await
        .unwrap();
    let state = sql::lock_check_state(&mut conn, check(PING)).await.unwrap();
    sql::write_check_state(&mut conn, check(PING), &state)
        .await
        .unwrap();
    let outbox = NewOutbox {
        id: OutboxId::from(Uuid::now_v7()),
        channel: id,
        check: Some(check(PING)),
        kind: NotificationKind::Down,
        payload: serde_json::json!({}),
    };
    sql::insert_outbox(&mut conn, &outbox, at).await.unwrap();
    let mut settings_row = sql::installation_settings(&mut *conn).await.unwrap();
    settings_row.certificate_days = vec![30, 7];
    sql::write_installation_settings(&mut conn, &settings_row)
        .await
        .unwrap();
    drop(conn);
    let delivery = Delivery {
        id: Uuid::now_v7().into(),
        channel: Some(id),
        channel_name: "ops".to_owned(),
        kind: NotificationKind::Down,
        at,
        ok: false,
        message: Some("The target did not answer in 8 seconds.".to_owned()),
    };
    sql::insert_delivery(&runtime, &delivery).await.unwrap();
    create_partitions_ahead(&runtime, at).await.unwrap();

    let claimed = sql::claim_due(&runtime, at, TimeDelta::minutes(5), 5)
        .await
        .unwrap();
    let last = sql::last_deliveries(&runtime, &[id]).await.unwrap();
    let page = sql::deliveries_page(&runtime, None, None, 10)
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(last[&id], delivery);
    assert_eq!(page, [delivery]);
    assert!(sql::mark_sent(&runtime, claimed[0].id, at).await.unwrap());
    assert_eq!(delete_finished_outbox(&runtime, at).await.unwrap(), 0);
    let mut conn = runtime.acquire().await.unwrap();
    assert!(sql::delete_channel(&mut conn, id).await.unwrap());
    assert_eq!(state, CheckState::default());
}

#[sqlx::test(migrations = "./migrations")]
async fn db_009_runtime_role_cannot_create_or_drop_a_partition_directly(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    create_partitions_ahead(&runtime, now(&pool).await)
        .await
        .unwrap();
    let today: String = sqlx::query_scalar(
        "select 'notifications.deliveries_' || to_char(now() at time zone 'UTC', 'YYYYMMDD')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let statements = [
        "create table notifications.deliveries_20990101 partition of notifications.deliveries \
         for values from ('2099-01-01T00:00:00Z') to ('2099-01-02T00:00:00Z')"
            .to_owned(),
        "drop table notifications.deliveries".to_owned(),
        format!("drop table {today}"),
        format!("alter table notifications.deliveries detach partition {today}"),
        "create table notifications.extra (id int)".to_owned(),
    ];

    for sql in statements {
        assert_eq!(
            refusal(&runtime, &sql).await.as_deref(),
            Some("42501"),
            "{sql}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn db_009_the_partition_function_refuses_a_day_outside_its_window(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;

    let far = "select notifications.ensure_day_partition(now() + interval '9 days')";
    let ancient = "select notifications.ensure_day_partition(now() - interval '40 days')";

    assert_eq!(refusal(&runtime, far).await.as_deref(), Some("22023"));
    assert_eq!(refusal(&runtime, ancient).await.as_deref(), Some("22023"));
}

#[sqlx::test(migrations = "./migrations")]
async fn db_009_the_runtime_role_owns_nothing_in_the_schema(pool: PgPool) {
    let owned: i64 = sqlx::query_scalar(
        "select (select count(*) from pg_class c where c.relnamespace = 'notifications'::regnamespace \
                 and c.relowner = 'anima_runtime'::regrole) \
              + (select count(*) from pg_proc p where p.pronamespace = 'notifications'::regnamespace \
                 and p.proowner = 'anima_runtime'::regrole) \
              + (select count(*) from pg_namespace n where n.nspname = 'notifications' \
                 and n.nspowner = 'anima_runtime'::regrole)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let definer: i64 = sqlx::query_scalar(
        "select count(*) from pg_proc p where p.pronamespace = 'notifications'::regnamespace \
         and p.prosecdef and p.proconfig = array['search_path=pg_catalog, pg_temp']",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(owned, 0);
    assert_eq!(definer, 2);
}
