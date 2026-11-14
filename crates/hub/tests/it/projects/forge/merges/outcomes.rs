//! K-E1 Sections 13.6 and 13.7, decisions items 2 and 7: every answer to a sent merge reaches
//! the client as its K7 code and fixed words, never GitHub's text, and leaves an audit entry
//! with its outcome; a merge whose answer was lost is unknown, recorded, and sent once.

use serde_json::json;
use sqlx::PgPool;

use super::{clean, merge, methods, sent};
use crate::{
    hub::error,
    projects::forge::{
        BASE,
        changes::{audits, events},
        forge, reply,
    },
};

const GITHUB_TEXT: &str = "GITHUB-MERGE-TEXT-7731";

const UNKNOWN: &str =
    "GitHub did not answer. The change may have been made: look before you try again.";

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_each_answer_to_a_merge_has_its_code_words_and_recorded_outcome(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    let refused = "GitHub refused the merge.";
    let cases: [(u16, &[(&'static str, &str)], &str, &str, &str); 7] = [
        (405, &[], refused, "CONFLICT", "refused"),
        (
            409,
            &[],
            "The pull request changed since you looked. Read it again.",
            "CONFLICT",
            "refused",
        ),
        (422, &[], refused, "CONFLICT", "refused"),
        (
            403,
            &[],
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            "FORBIDDEN",
            "refused",
        ),
        (
            404,
            &[],
            "GitHub has no such pull request, or the App may not change it.",
            "NOT_FOUND",
            "refused",
        ),
        (500, &[], UNKNOWN, "UNAVAILABLE", "unknown"),
        (
            429,
            &[("retry-after", "600")],
            "GitHub limits requests until 12:10.",
            "UNAVAILABLE",
            "limited",
        ),
    ];
    for (n, (status, headers, words, code, outcome)) in (1_u64..).zip(cases) {
        forge.ok(&format!("/pulls/{n}"), &clean(n));
        let said = json!({ "message": GITHUB_TEXT }).to_string();
        let path = format!("{BASE}/pulls/{n}/merge");
        (forge.kit.stub).reply_to("PUT", &path, reply(status, headers, &said));
        forge.clock.advance(2);

        let response = merge(&forge, n, &json!({})).await;

        assert_eq!(
            error(&response),
            (words.to_owned(), code.to_owned()),
            "{status}"
        );
        assert!(!response.to_string().contains(GITHUB_TEXT), "{response}");
        assert_eq!(sent(&forge, n).len(), 1, "{status}: sent once");
        let entries = audits(&pool).await;
        let entry = entries.last().unwrap();
        assert_eq!(entries.len(), usize::try_from(n).unwrap(), "one entry each");
        assert_eq!(entry["action"], "merge");
        assert_eq!(entry["change"]["outcome"], outcome, "{status}");
        assert_eq!(entry["change"]["target"], format!("pull_request#{n}"));
        assert!(!entry.to_string().contains(GITHUB_TEXT));
    }
    let severities: Vec<String> = events(&pool).await.into_iter().map(|e| e.0).collect();
    assert_eq!(severities, vec!["warning".to_owned(); 7]);
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_dropped_answer_to_a_merge_is_unknown_recorded_and_sent_once(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);
    forge.ok("/pulls/5", &clean(5));
    forge.kit.stub.drop_answer(&format!("{BASE}/pulls/5/merge"));

    let response = merge(&forge, 5, &json!({})).await;

    assert_eq!(
        error(&response),
        (UNKNOWN.to_owned(), "UNAVAILABLE".to_owned())
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(sent(&forge, 5).len(), 1, "never sent again");
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["change"]["outcome"], "unknown");
    assert_eq!(
        events(&pool).await,
        [(
            "warning".to_owned(),
            "Project widgets: merge_pull_request on pull_request#5, sent with no answer from \
             GitHub: it may have been made."
                .to_owned()
        )]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_pull_request_github_does_not_have_is_not_found_and_sends_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    methods(&forge);

    let response = merge(&forge, 4040, &json!({})).await;

    assert_eq!(
        error(&response),
        (
            "GitHub has no such pull request, or the App may not read it.".to_owned(),
            "NOT_FOUND".to_owned()
        )
    );
    assert!(sent(&forge, 4040).is_empty());
    assert!(audits(&pool).await.is_empty());
}
