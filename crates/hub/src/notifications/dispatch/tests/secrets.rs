//! No secret of a channel reaches a delivery, an event or the process log, whatever the
//! outcome of the send (LOG-007, NOT-002, SEC-005).

use sqlx::PgPool;

use super::{
    DISCORD, NO_CONTENT, answering, apprise, apprise_key, assert_no_secret, channel, deliveries,
    dispatcher, email, ntfy, queue, recorded, start, target, warnings, webhook,
};
use crate::{
    events::process_log::tests::capture::capture_log,
    notifications::{DeliveryReason, egress::tests::servers::Behaviour},
};

const NOT_FOUND: &[u8] = b"HTTP/1.1 404 synthetic-header-secret\r\nContent-Length: 21\r\n\r\n\
synthetic-path-secret";

#[sqlx::test(migrations = "./migrations")]
async fn log_007_no_secret_is_logged_recorded_or_delivered_by_a_failing_send(pool: PgPool) {
    let (capture, _guard) = capture_log("trace");
    tracing::info!("log_007 capture marker");
    let (refusing, egress) = answering(NOT_FOUND).await;
    let (mail, mail_egress) = target(Behaviour::SmtpRefusingLogin).await;
    let t0 = start(&pool).await;
    for (name, kind) in [
        ("hooks", webhook(&refusing)),
        ("alerts", ntfy(&refusing)),
        ("apprise", apprise(&refusing, &[DISCORD])),
        ("keyed", apprise_key(&refusing)),
    ] {
        let id = channel(&pool, name, kind).await;
        queue(&pool, id, None, t0).await;
    }
    dispatcher(&pool, egress).tick(t0).await;
    let mailed = channel(&pool, "mail", email(&mail)).await;
    queue(&pool, mailed, None, t0).await;

    dispatcher(&pool, mail_egress).tick(t0).await;

    let logged = deliveries(&pool).await;
    assert_eq!(logged.len(), 5);
    assert!(logged.iter().all(|delivery| !delivery.ok));
    assert!(
        logged
            .iter()
            .any(|delivery| delivery.reason == Some(DeliveryReason::SmtpRefused))
    );
    assert_eq!(warnings(&pool).await.len(), 5);
    assert_no_secret(&format!("{logged:?}"));
    assert_no_secret(&recorded(&pool).await);
    let log = capture.text();
    assert!(log.contains("log_007 capture marker"), "{log}");
    assert!(log.contains("a notification was not delivered"), "{log}");
    assert_no_secret(&log);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_no_secret_is_logged_by_a_send_that_succeeds(pool: PgPool) {
    let (capture, _guard) = capture_log("trace");
    tracing::info!("log_007 capture marker");
    let (server, egress) = answering(NO_CONTENT).await;
    let t0 = start(&pool).await;
    for (name, kind) in [("hooks", webhook(&server)), ("alerts", ntfy(&server))] {
        let id = channel(&pool, name, kind).await;
        queue(&pool, id, None, t0).await;
    }

    dispatcher(&pool, egress).tick(t0).await;

    assert!(deliveries(&pool).await.iter().all(|delivery| delivery.ok));
    let log = capture.text();
    assert!(log.contains("log_007 capture marker"), "{log}");
    assert_no_secret(&log);
}
