//! K-E1 Section 13.6: a change refused before sending (its input, the repository's access,
//! Projects off) sends nothing to GitHub and records nothing. A `VALIDATION` error names the
//! field, never its value (LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{COMMENT, CREATE, SET_OPEN, audits};
use crate::{
    hub::error,
    projects::forge::{BASE, Forge, forge, pull_as_issue},
    sources::set_projects,
};

const MARK: &str = "REFUSED-VALUE-3301";

/// Requests below the repository that would change something.
fn changes_sent(forge: &Forge) -> usize {
    forge.kit.stub.with(|s| {
        s.heard
            .iter()
            .filter(|h| h.method != "GET" && h.path.starts_with(BASE))
            .count()
    })
}

/// The fields a `VALIDATION` error names.
fn fields(response: &Value) -> Vec<String> {
    assert_eq!(error(response).1, "VALIDATION", "{response}");
    let listed = response["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap_or_else(|| panic!("no fields: {response}"));
    listed
        .iter()
        .map(|f| f["field"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_bad_input_is_validation_naming_the_field_and_sends_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/issues/12", &pull_as_issue(12));
    let labels: Vec<String> = (0..21).map(|n| format!("l{n}")).collect();
    let cases = [
        (CREATE, json!({ "title": "" }), vec!["title"]),
        (
            CREATE,
            json!({ "title": format!("{MARK}{}", "t".repeat(257)) }),
            vec!["title"],
        ),
        (
            CREATE,
            json!({ "title": "Fine", "body": format!("{MARK}\u{0}") }),
            vec!["body"],
        ),
        (
            CREATE,
            json!({ "title": "Fine", "labels": labels }),
            vec!["labels"],
        ),
        (
            CREATE,
            json!({ "title": format!("{MARK}\n"), "labels": [""] }),
            vec!["title", "labels"],
        ),
        (COMMENT, json!({ "number": 5, "body": "" }), vec!["body"]),
        (
            COMMENT,
            json!({ "number": 0, "body": format!("{MARK}\u{0}") }),
            vec!["number", "body"],
        ),
        (
            SET_OPEN,
            json!({ "number": 3, "open": true, "reason": "COMPLETED" }),
            vec!["reason"],
        ),
        (
            SET_OPEN,
            json!({ "number": 12, "open": false }),
            vec!["number"],
        ),
    ];
    for (query, variables, named) in cases {
        let response = forge.ask(query, variables.clone()).await;
        assert_eq!(fields(&response), named, "{variables}");
        assert!(!response.to_string().contains(MARK), "{response}");
    }
    assert_eq!(changes_sent(&forge), 0, "nothing was sent");
    assert!(audits(&pool).await.is_empty(), "nothing was recorded");
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_repository_that_is_not_ready_or_projects_off_sends_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    let calls = [
        (CREATE, json!({ "title": "Printer jams" })),
        (COMMENT, json!({ "number": 5, "body": "Seen it." })),
        (SET_OPEN, json!({ "number": 3, "open": false })),
    ];
    sqlx::query("UPDATE projects.repositories SET left_out_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    for (query, variables) in &calls {
        let response = forge.ask(query, variables.clone()).await;
        assert_eq!(
            error(&response),
            (
                "This repository is left out. Restore it to read it.".to_owned(),
                "CONFLICT".to_owned()
            ),
            "{query}"
        );
    }
    sqlx::query("UPDATE projects.repositories SET left_out_at = NULL")
        .execute(&pool)
        .await
        .unwrap();
    set_projects(&pool, false).await;
    for (query, variables) in &calls {
        let response = forge.ask(query, variables.clone()).await;
        let (message, code) = error(&response);
        assert_eq!(code, "CONFLICT", "{query}");
        assert!(message.contains("Projects is off"), "{message}");
    }
    assert_eq!(changes_sent(&forge), 0);
    assert_eq!(forge.live_reads(), 0, "GitHub was never asked");
    assert!(audits(&pool).await.is_empty(), "nothing");
}
