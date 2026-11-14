//! LOG-007, PRJ-018: neither the installation token, a reply body nor a change's body reaches a
//! process log line, a failure's words or its `Debug`, or a reply's `Debug`.

use anima_hub::{
    engine::sources::{Keep, ask, change},
    fleet::events::process_log,
};
use sqlx::PgPool;

use super::{COMMENT_PATH, PULLS, comment, get, live_kit, ok, reply, striding};
use crate::{logs::Capture, sources::unreachable};

const REPLY_MARKER: &str = "REPLY-BODY-MARKER-3333";
const CHANGE_MARKER: &str = "CHANGE-BODY-MARKER-4444";

#[sqlx::test(migrations = "./migrations")]
async fn log_007_the_token_and_the_bodies_are_in_no_log_line_or_error(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let (kit, id) = live_kit(&pool).await;
    let token = kit.stub.token();
    let echo = format!("{{\"message\":\"{REPLY_MARKER} {token}\"}}");
    let s = striding(&kit, kit.reach.transport.clone());
    let mut texts = Vec::new();
    let mut note = |text: String| texts.push(text);

    kit.stub.reply(PULLS, ok(&echo));
    note(format!(
        "{:?}",
        ask(&s, id, &get(PULLS), Keep::for_seconds(30)).await
    ));
    for status in [401, 403, 429, 502] {
        let path = format!("/status/{status}");
        kit.stub
            .reply(&path, reply(status, &[("retry-after", "1")], &echo));
        let asked = ask(&s, id, &get(&path), Keep::never()).await;
        note(format!("{asked:?}"));
        if let Err(failure) = asked {
            note(failure.to_string());
        }
    }
    kit.stub.reply(COMMENT_PATH, reply(422, &[], &echo));
    let body = format!("{CHANGE_MARKER} {token}");
    note(format!("{:?}", change(&s, id, &comment(&body)).await));
    kit.stub.reply(COMMENT_PATH, reply(401, &[], &echo));
    note(format!("{:?}", change(&s, id, &comment(&body)).await));
    let failed = change(&striding(&kit, unreachable()), id, &comment(&body)).await;
    note(format!("{failed:?}"));
    if let Err(failure) = failed {
        note(failure.to_string());
    }
    drop(note);

    let log = capture.text();
    assert!(!log.is_empty(), "the log was captured");
    for needle in [token.as_str(), REPLY_MARKER, CHANGE_MARKER] {
        assert!(!log.contains(needle), "{needle} is in the log");
        for text in &texts {
            assert!(!text.contains(needle), "{needle} is in {text}");
        }
    }
}
