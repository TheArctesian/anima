//! The migrations: what they create, what they refuse and what they cascade (K2, DB-010).

use sqlx::PgPool;
use uuid::Uuid;

use super::{PING, SSH_PORT, channel, check, count, fields, now, refusal};
use crate::{
    fleet::notifications::{
        CheckSettings, CheckState, Delivery, Error, NewOutbox, NotificationKind, OutboxId,
        store as sql,
    },
    ids::ChannelId,
};

#[sqlx::test(migrations = "./migrations")]
async fn not_007_settings_are_seeded_with_the_default_days_and_stay_one_row(pool: PgPool) {
    let settings = sql::installation_settings(&pool).await.unwrap();

    assert_eq!(settings.certificate_days, [21, 14, 7]);
    assert!(settings.title_template.contains("{device}"));
    assert!(settings.body_template.contains("{status}"));
    assert_eq!(
        refusal(
            &pool,
            "insert into notifications.settings (id) values (false)"
        )
        .await
        .as_deref(),
        Some("23514")
    );
    assert_eq!(
        refusal(
            &pool,
            "insert into notifications.settings (id) values (true)"
        )
        .await
        .as_deref(),
        Some("23505")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn not_006_certificate_days_hold_one_to_five_values_of_one_to_365(pool: PgPool) {
    for days in [
        "'{}'",
        "'{30,20,10,5,3,2}'",
        "'{0}'",
        "'{366}'",
        "'{null,7}'",
    ] {
        let sql = format!("update notifications.settings set certificate_days = {days}");
        assert_eq!(
            refusal(&pool, &sql).await.as_deref(),
            Some("23514"),
            "{days}"
        );
    }
    let accepted = "update notifications.settings set certificate_days = '{365,1}'";
    assert_eq!(refusal(&pool, accepted).await, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_001_kinds_are_the_lower_snake_notification_kinds(pool: PgPool) {
    let id = channel(&pool, "ops").await;
    let insert = |kind: &str| {
        format!(
            "insert into notifications.outbox (id, channel_id, kind, payload) \
             values (gen_random_uuid(), '{id}', '{kind}', '{{}}')"
        )
    };

    for kind in [
        NotificationKind::Down,
        NotificationKind::Up,
        NotificationKind::StillDown,
        NotificationKind::CertificateExpiry,
        NotificationKind::DomainExpiry,
        NotificationKind::Test,
    ] {
        assert_eq!(refusal(&pool, &insert(kind.as_str())).await, None);
    }
    assert_eq!(
        refusal(&pool, &insert("DOWN")).await.as_deref(),
        Some("23514")
    );
    assert_eq!(
        refusal(&pool, "update notifications.channels set kind = 'sms'")
            .await
            .as_deref(),
        Some("23514")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_the_event_log_accepts_source_notification_and_still_metrics(pool: PgPool) {
    sqlx::query("select events.ensure_day_partition('event_log', now())")
        .execute(&pool)
        .await
        .unwrap();

    for source in [
        "check",
        "session",
        "device",
        "access",
        "hub",
        "metrics",
        "notification",
    ] {
        let sql = format!(
            "insert into events.event_log (id, at, severity, source, message) \
             values (gen_random_uuid(), now(), 'warning', '{source}', 'm')"
        );
        assert_eq!(refusal(&pool, &sql).await, None, "{source}");
    }
    let unknown = "insert into events.event_log (id, at, severity, source, message) \
                   values (gen_random_uuid(), now(), 'warning', 'mail', 'm')";
    assert_eq!(refusal(&pool, unknown).await.as_deref(), Some("23514"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_003_deleting_a_check_removes_its_links_settings_and_state(pool: PgPool) {
    let ops = channel(&pool, "ops").await;
    let mut conn = pool.acquire().await.unwrap();
    for id in [check(PING), check(SSH_PORT)] {
        sql::set_check_channels(&mut conn, id, &[ops])
            .await
            .unwrap();
        let settings = CheckSettings {
            resend_every_beats: 5,
        };
        sql::write_check_settings(&mut conn, id, settings)
            .await
            .unwrap();
        sql::write_check_state(&mut conn, id, &CheckState::default())
            .await
            .unwrap();
    }

    sqlx::query("delete from checks.checks where id = $1")
        .bind(check(PING))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(count(&pool, "check_channels").await, 1);
    assert_eq!(count(&pool, "check_settings").await, 1);
    assert_eq!(count(&pool, "check_state").await, 1);
    assert_eq!(count(&pool, "channels").await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_002_deleting_a_channel_removes_links_and_queue_but_keeps_deliveries(pool: PgPool) {
    let doomed = channel(&pool, "doomed").await;
    let kept = channel(&pool, "kept").await;
    let at = now(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    sql::set_check_channels(&mut conn, check(PING), &[doomed, kept])
        .await
        .unwrap();
    for id in [doomed, kept] {
        let row = NewOutbox {
            id: OutboxId::from(Uuid::now_v7()),
            channel: id,
            check: Some(check(PING)),
            kind: NotificationKind::Down,
            payload: serde_json::json!({}),
        };
        sql::insert_outbox(&mut conn, &row, at).await.unwrap();
    }
    let delivery = Delivery {
        id: Uuid::now_v7().into(),
        channel: Some(doomed),
        channel_name: "doomed".to_owned(),
        kind: NotificationKind::Down,
        at,
        ok: true,
        reason: None,
        code: None,
    };
    sql::insert_delivery(&mut pool.acquire().await.unwrap(), &delivery)
        .await
        .unwrap();

    assert!(sql::delete_channel(&mut conn, doomed).await.unwrap());

    assert_eq!(count(&pool, "check_channels").await, 1);
    assert_eq!(count(&pool, "outbox").await, 1);
    assert_eq!(count(&pool, "deliveries").await, 1);
    let remaining = sql::check_channels(&pool, &[check(PING)]).await.unwrap();
    assert_eq!(remaining[&check(PING)], [kept]);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_channel_name_is_unique(pool: PgPool) {
    channel(&pool, "ops").await;
    let mut conn = pool.acquire().await.unwrap();

    let error = sql::insert_channel(
        &mut conn,
        ChannelId::from(Uuid::now_v7()),
        &fields("ops"),
        now(&pool).await,
    )
    .await
    .unwrap_err();

    assert!(matches!(error, Error::NameTaken));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_an_ok_delivery_has_no_reason_and_a_failed_one_has_one(pool: PgPool) {
    sqlx::query("select notifications.ensure_day_partition(now())")
        .execute(&pool)
        .await
        .unwrap();
    let insert = |ok: bool, reason: &str| {
        format!(
            "insert into notifications.deliveries (id, channel_name, kind, at, ok, reason) \
             values (gen_random_uuid(), 'ops', 'test', now(), {ok}, {reason})"
        )
    };

    assert_eq!(refusal(&pool, &insert(true, "null")).await, None);
    assert_eq!(refusal(&pool, &insert(false, "'timed_out'")).await, None);
    assert_eq!(
        refusal(&pool, &insert(true, "'timed_out'"))
            .await
            .as_deref(),
        Some("23514")
    );
    assert_eq!(
        refusal(&pool, &insert(false, "null")).await.as_deref(),
        Some("23514")
    );
    assert_eq!(
        refusal(&pool, &insert(false, "'free text'"))
            .await
            .as_deref(),
        Some("23514")
    );
}
