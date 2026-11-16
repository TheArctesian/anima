//! LOG-007, PRJ-028: the title, body and branch of a pull request, what the caller typed and
//! what GitHub answered reach no process log line, no error message, no result message, no
//! audit entry and no event.

use anima_hub::fleet::events::process_log;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{MERGE_MANY, clean, merge, merged, methods, reads};
use crate::{
    logs::Capture,
    projects::forge::{
        changes::{answer, audits},
        forge, sha,
    },
};

const WRITTEN: &str = "PULL-REQUEST-TEXT-6194";
const ANSWERED: &str = "GITHUB-ANSWERED-2087";
const TYPED: &str = "CALLER-TYPED-5512";

/// Pull request `number` with GitHub's text marked.
fn marked(mut one: Value) -> Value {
    one["title"] = json!(format!("{WRITTEN} title"));
    one["body"] = json!(format!("{WRITTEN} body"));
    one["head"]["ref"] = json!(format!("{WRITTEN}-branch"));
    one
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_pull_requests_text_and_githubs_answer_are_never_kept(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let forge = forge(&pool).await;
    methods(&forge);
    reads(&forge, 1, &marked(clean(1)), &marked(merged(1)));
    let made = json!({ "sha": sha(0xbeef), "merged": true, "message": ANSWERED });
    answer(&forge, ("PUT", "/pulls/1/merge"), 200, &made);
    forge.ok("/pulls/2", &marked(clean(2)));
    let refusal = json!({ "message": ANSWERED, "documentation_url": ANSWERED });
    answer(&forge, ("PUT", "/pulls/2/merge"), 405, &refusal);
    forge.ok("/pulls/3", &marked(clean(3)));
    answer(&forge, ("PUT", "/pulls/3/merge"), 409, &refusal);

    let done = merge(&forge, 1, &json!({ "forkConfirmed": TYPED })).await;
    assert!(done["errors"].is_null(), "{done}");
    forge.clock.advance(2);
    let refused = merge(&forge, 2, &json!({})).await;
    let wrong = merge(&forge, 3, &json!({ "expectedHeadSha": sha(7) })).await;
    let bulk = forge
        .kit
        .post(
            MERGE_MANY,
            json!({ "pullRequests": [{ "repository": forge.repository, "number": 3,
                "expectedHeadSha": sha(3) }], "method": "MERGE" }),
        )
        .await;
    assert_eq!(
        bulk["data"]["mergePullRequests"][0]["outcome"], "HEAD_MOVED",
        "{bulk}"
    );
    let answers = format!("{refused} {wrong} {bulk}");
    for mark in [WRITTEN, ANSWERED, TYPED] {
        assert!(!answers.contains(mark), "{mark} in an answer: {answers}");
    }

    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 3, "{entries:?}");
    let events: Vec<String> =
        sqlx::query_scalar("SELECT to_jsonb(e)::text FROM events.event_log e")
            .fetch_all(&pool)
            .await
            .unwrap();
    let kept = format!("{entries:?} {events:?}");
    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    for mark in [WRITTEN, ANSWERED, TYPED] {
        assert!(!kept.contains(mark), "{mark} was kept");
        assert!(!log.contains(mark), "{mark} is in the log");
    }
}
