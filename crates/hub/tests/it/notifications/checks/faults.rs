//! A notification fault never loses a heartbeat: an unreadable notification state or a
//! statement that fails inside observe keeps the beat, and the log names only the check's id
//! and the fault word (R-104, LOG-007).

use sqlx::PgPool;

use super::heartbeats::{PVE_01, kinds, rig};
use crate::notifications::{harness::holds_no_secret, unreadable::logged};

/// Expiry marks this version cannot read: a later version or a database writer put them there.
async fn break_state(pool: &PgPool, check: &str) {
    sqlx::query(
        "INSERT INTO notifications.check_state (check_id, expiry_marks) \
         VALUES ($1::text::uuid, '[{\"not-a-mark\": \"state-s3cr3t\"}]')",
    )
    .bind(check)
    .execute(pool)
    .await
    .unwrap();
}

/// Asserts that the log names the check and the fault word and nothing the operator set up.
fn names_only_the_check(log: &str, check: &str, fault: &str, hidden: &[&str]) {
    let line = log
        .lines()
        .find(|line| line.contains("could not be queued"))
        .unwrap_or_else(|| panic!("no fault logged: {log}"));
    assert!(
        line.contains(check),
        "the fault line names no check: {line}"
    );
    assert!(
        line.contains(fault),
        "the fault line names no fault: {line}"
    );
    for text in hidden {
        assert!(!log.contains(text), "the log holds {text}");
    }
    holds_no_secret("the process log", log);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_104_an_unreadable_notification_state_keeps_the_beat_and_logs_only_the_check_id(
    pool: PgPool,
) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let (id, path) = rig.push_check(PVE_01, "Backup job", &[&a]).await;
    break_state(&pool, &id).await;

    let (log, ()) = logged(rig.push(&path, "status=down&msg=msg-s3cr3t")).await;

    assert_eq!(rig.beats(&id).await, ["down"], "the beat is kept");
    assert_eq!(rig.queued(&id).await, []);
    let token = path.trim_start_matches("/push/");
    let hidden = [
        a.as_str(),
        "Alpha",
        "pve-01",
        "Backup job",
        "msg-s3cr3t",
        "state-s3cr3t",
        token,
    ];
    names_only_the_check(&log, &id, "unreadable", &hidden);
}

/// Makes every outbox insert fail with an SQL error, which aborts the statement's
/// transaction unless a savepoint contains it.
async fn fail_outbox_inserts(pool: &PgPool) {
    for statement in [
        "CREATE FUNCTION notifications.refuse_outbox() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'outbox-s3cr3t'; END $$",
        "CREATE TRIGGER refuse_outbox BEFORE INSERT ON notifications.outbox \
         FOR EACH ROW EXECUTE FUNCTION notifications.refuse_outbox()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_104_a_statement_that_fails_inside_observe_keeps_the_beat_and_its_state(pool: PgPool) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let (id, path) = rig.push_check(PVE_01, "Backup job", &[&a]).await;
    fail_outbox_inserts(&pool).await;

    let (log, ()) = logged(rig.push(&path, "status=down")).await;

    assert_eq!(rig.beats(&id).await, ["down"], "the beat is kept");
    assert_eq!(rig.queued(&id).await, []);
    names_only_the_check(
        &log,
        &id,
        "database",
        &[a.as_str(), "Alpha", "pve-01", "Backup job", "outbox-s3cr3t"],
    );
    let notified: Option<String> = sqlx::query_scalar(
        "SELECT notified_status FROM notifications.check_state WHERE check_id = $1::text::uuid",
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .unwrap()
    .flatten();
    assert_eq!(
        notified, None,
        "the decision's state rolled back with its rows"
    );

    sqlx::query("DROP TRIGGER refuse_outbox ON notifications.outbox")
        .execute(&pool)
        .await
        .unwrap();
    rig.push(&path, "status=down").await;
    assert_eq!(
        kinds(&rig.queued(&id).await),
        ["down"],
        "the Down is still announced"
    );
}
