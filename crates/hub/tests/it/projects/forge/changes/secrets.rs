//! LOG-007, PRJ-028: what the operator wrote (title, body, labels, comment) and what GitHub
//! answered reach no process log line, no error message, no audit entry and no event.

use anima_hub::fleet::events::process_log;
use serde_json::json;
use sqlx::PgPool;

use super::{COMMENT, CREATE, SET_OPEN, answer, audits};
use crate::{
    logs::Capture,
    projects::forge::{forge, issue},
};

const WRITTEN: &str = "OPERATOR-WROTE-5521";
const ANSWERED: &str = "GITHUB-ANSWERED-8814";

#[sqlx::test(migrations = "./migrations")]
async fn log_007_the_text_of_a_change_and_githubs_answer_are_never_kept(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let forge = forge(&pool).await;
    let github = json!({ "number": 7, "title": ANSWERED, "message": ANSWERED });
    answer(&forge, "POST", "/issues", 201, &github);
    let mut made = issue(7);
    made["body"] = json!(format!("{WRITTEN} body"));
    forge.ok("/issues/7", &made);
    let refusal = json!({ "message": ANSWERED, "errors": [{ "value": WRITTEN }] });
    answer(&forge, "POST", "/issues/5/comments", 422, &refusal);
    forge.ok("/issues/3", &issue(3));
    answer(
        &forge,
        "PATCH",
        "/issues/3",
        500,
        &json!({ "message": ANSWERED }),
    );

    let created = forge
        .ask(
            CREATE,
            json!({ "title": format!("{WRITTEN} title"), "body": format!("{WRITTEN} body"),
                "labels": [format!("{WRITTEN} label")] }),
        )
        .await;
    assert!(created["errors"].is_null(), "{created}");
    forge.clock.advance(2);
    let commented = forge
        .ask(
            COMMENT,
            json!({ "number": 5, "body": format!("{WRITTEN} comment") }),
        )
        .await;
    forge.clock.advance(2);
    let closed = forge
        .ask(SET_OPEN, json!({ "number": 3, "open": false }))
        .await;
    for failed in [&commented, &closed] {
        let errors = failed["errors"].to_string();
        assert!(
            !errors.contains(WRITTEN) && !errors.contains(ANSWERED),
            "{errors}"
        );
    }

    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 3, "{entries:?}");
    let events: Vec<String> =
        sqlx::query_scalar("SELECT to_jsonb(e)::text FROM events.event_log e")
            .fetch_all(&pool)
            .await
            .unwrap();
    let kept = format!("{entries:?} {events:?}");
    assert!(!kept.contains(WRITTEN), "the operator's text was kept");
    assert!(!kept.contains(ANSWERED), "GitHub's answer was kept");
    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    assert!(!log.contains(WRITTEN), "the operator's text is in the log");
    assert!(!log.contains(ANSWERED), "GitHub's answer is in the log");
}
