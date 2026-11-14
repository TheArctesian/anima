//! K-E1 Sections 13.6 and 13.7, decisions item 7: every answer to a sent change reaches the
//! client as its K7 code and fixed words, never GitHub's text, and leaves an audit entry with
//! its outcome; a change whose answer was lost is unknown, recorded, and sent once.

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CREATE, audits, events, sent};
use crate::{
    hub::{error, start},
    projects::forge::{BASE, forge, reply},
};

const GITHUB_TEXT: &str = "GITHUB-OWN-TEXT-6620";

fn said() -> String {
    json!({ "message": GITHUB_TEXT }).to_string()
}

const UNKNOWN: &str =
    "GitHub did not answer. The change may have been made: look before you try again.";

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_each_answer_has_its_code_words_and_recorded_outcome(pool: PgPool) {
    let forge = forge(&pool).await;
    let reset = (start() + TimeDelta::minutes(15)).timestamp().to_string();
    let limit = [
        ("x-ratelimit-remaining", "0"),
        ("x-ratelimit-reset", reset.as_str()),
    ];
    let cases: [(u16, &[(&str, &str)], &str, &str, &str); 8] = [
        (
            401,
            &[],
            "GitHub refused the key. Connect the account again.",
            "CONFLICT",
            "refused",
        ),
        (
            403,
            &[],
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            "FORBIDDEN",
            "refused",
        ),
        (
            403,
            &limit,
            "GitHub limits requests until 12:15.",
            "UNAVAILABLE",
            "limited",
        ),
        (
            404,
            &[],
            "GitHub has no such issue, or the App may not change it.",
            "NOT_FOUND",
            "refused",
        ),
        (
            410,
            &[],
            "Issues are off for this repository.",
            "CONFLICT",
            "refused",
        ),
        (
            422,
            &[],
            "GitHub refused a value of the change. Nothing was changed.",
            "VALIDATION",
            "refused",
        ),
        (
            429,
            &[("retry-after", "600")],
            "GitHub limits requests until 12:10.",
            "UNAVAILABLE",
            "limited",
        ),
        (500, &[], UNKNOWN, "UNAVAILABLE", "unknown"),
    ];
    for (n, (status, headers, words, code, outcome)) in cases.into_iter().enumerate() {
        let path = format!("{BASE}/issues");
        (forge.kit.stub).reply_to("POST", &path, reply(status, headers, &said()));
        forge.clock.advance(2);
        let response = forge.ask(CREATE, json!({ "title": "Printer jams" })).await;
        assert_eq!(
            error(&response),
            (words.to_owned(), code.to_owned()),
            "{status}"
        );
        assert!(!response.to_string().contains(GITHUB_TEXT), "{response}");
        let entries = audits(&pool).await;
        assert_eq!(entries.len(), n + 1, "one entry for each sent change");
        let change = &entries[n]["change"];
        assert_eq!(change["outcome"], outcome, "{status}");
        assert_eq!(change["function"], "create_issue");
        assert_eq!(change["target"], "issue#new");
        assert!(!entries[n].to_string().contains(GITHUB_TEXT));
    }
    assert_eq!(sent(&forge, "POST", "/issues").len(), 8, "each sent once");
    let severities: Vec<String> = events(&pool).await.into_iter().map(|e| e.0).collect();
    assert_eq!(severities, vec!["warning".to_owned(); 8]);
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_dropped_answer_is_unknown_recorded_and_sent_once(pool: PgPool) {
    let forge = forge(&pool).await;
    forge
        .kit
        .stub
        .drop_answer(&format!("{BASE}/issues/5/comments"));

    let response = forge
        .ask(
            super::COMMENT,
            json!({ "number": 5, "body": "Seen it too." }),
        )
        .await;

    assert_eq!(
        error(&response),
        (UNKNOWN.to_owned(), "UNAVAILABLE".to_owned())
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(sent(&forge, "POST", "/issues/5/comments").len(), 1);
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["change"]["outcome"], "unknown");
    assert_eq!(entries[0]["change"]["target"], "issue#5");
    assert_eq!(
        events(&pool).await,
        [(
            "warning".to_owned(),
            "Project widgets: add_issue_comment on issue#5, sent with no answer from GitHub: \
             it may have been made."
                .to_owned()
        )]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_change_github_made_that_cannot_be_recorded_is_internal(pool: PgPool) {
    let forge = forge(&pool).await;
    let path = format!("{BASE}/issues");
    let made = reply(201, &[], &json!({ "number": 7 }).to_string());
    forge.kit.stub.reply_to("POST", &path, made);
    forge.kit.stub.hook(
        &path,
        "ALTER TABLE events.audit_log ADD CONSTRAINT no_repository_entries \
         CHECK (target_type <> 'repository')",
    );

    let response = forge.ask(CREATE, json!({ "title": "Printer jams" })).await;

    assert_eq!(error(&response).1, "INTERNAL", "{response}");
    assert_eq!(sent(&forge, "POST", "/issues").len(), 1, "never sent again");
    assert_eq!(audits(&pool).await, Vec::<Value>::new());
    assert!(events(&pool).await.is_empty());
}
