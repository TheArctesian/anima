//! OD-E14, PRJ-028: each issue change leaves exactly as K-E1 Section 13.3 says, returns K1's
//! shape read back from GitHub, leaves one audit entry and one event, and drops the held issue
//! answers so the next list read asks GitHub again.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{COMMENT, CREATE, LIST, SET_OPEN, answer, audits, body, closed, comment, events, sent};
use crate::projects::forge::{Forge, forge, issue};

/// Reads the issue list once, so a held answer is there to drop.
async fn listed(forge: &Forge) {
    forge.ok("/issues", &json!([issue(1)]));
    let response = forge.ask(LIST, json!({})).await;
    assert!(response["errors"].is_null(), "{response}");
}

/// The one audit entry, as stored, after a done change.
async fn one_entry(forge: &Forge) -> Value {
    let entries = audits(&forge.kit.pool).await;
    assert_eq!(entries.len(), 1, "{entries:?}");
    let entry = entries[0].clone();
    assert_eq!(entry["target_type"], "repository");
    assert_eq!(entry["target_id"], forge.repository.as_str());
    assert_eq!(entry["actor_kind"], "user");
    assert_eq!(entry["token_kind"], "login");
    assert_eq!(entry["change"]["repository"], "octo/widgets");
    assert_eq!(entry["change"]["outcome"], "done");
    entry
}

/// The list read after the change asks GitHub again: its held answer was dropped.
async fn asked_again(forge: &Forge) {
    let response = forge.ask(LIST, json!({})).await;
    assert!(response["errors"].is_null(), "{response}");
    assert_eq!(forge.heard("/issues").len(), 2, "the held list was dropped");
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_create_issue_posts_the_title_body_and_labels_and_returns_the_new_issue(
    pool: PgPool,
) {
    let forge = forge(&pool).await;
    listed(&forge).await;
    answer(&forge, ("POST", "/issues"), 201, &json!({ "number": 7 }));
    let mut made = issue(7);
    made["title"] = json!("Printer jams");
    made["body"] = json!("Steps to see it.");
    made["labels"] = json!([{ "name": "bug" }]);
    forge.ok("/issues/7", &made);

    let response = forge
        .ask(
            CREATE,
            json!({ "title": "Printer jams", "body": "Steps to see it.", "labels": ["bug"] }),
        )
        .await;

    let node = &response["data"]["createIssue"];
    assert_eq!(
        node,
        &json!({ "repository": forge.repository, "number": 7, "title": "Printer jams",
            "open": true, "closeReason": null, "authorLogin": "octo", "labels": ["bug"],
            "assignees": [], "comments": 0, "createdAt": node["createdAt"],
            "updatedAt": node["updatedAt"], "closedAt": null,
            "url": "https://github.com/octo/widgets/issues/7", "body": "Steps to see it.",
            "bodyCut": false }),
        "{response}"
    );
    let posts = sent(&forge, "POST", "/issues");
    assert_eq!(posts.len(), 1);
    assert_eq!(
        body(&posts[0]),
        json!({ "title": "Printer jams", "body": "Steps to see it.", "labels": ["bug"] })
    );
    let entry = one_entry(&forge).await;
    assert_eq!(entry["action"], "create");
    assert_eq!(entry["change"]["function"], "create_issue");
    assert_eq!(entry["change"]["target"], "issue#7");
    assert_eq!(
        events(&pool).await,
        [(
            "info".to_owned(),
            "Project widgets: create_issue on issue#7, done.".to_owned()
        )]
    );
    asked_again(&forge).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_comment_is_posted_and_read_back_from_the_last_page(pool: PgPool) {
    let forge = forge(&pool).await;
    listed(&forge).await;
    answer(
        &forge,
        ("POST", "/issues/5/comments"),
        201,
        &json!({ "id": 9001 }),
    );
    let mut commented = issue(5);
    commented["comments"] = json!(101);
    forge.ok("/issues/5", &commented);
    forge.ok(
        "/issues/5/comments",
        &json!([comment(9001, 5, "Seen it too.")]),
    );

    let response = forge
        .ask(COMMENT, json!({ "number": 5, "body": "Seen it too." }))
        .await;

    assert_eq!(
        response["data"]["addIssueComment"],
        json!({ "id": "9001", "authorLogin": "anima-app[bot]", "body": "Seen it too.",
            "bodyCut": false, "createdAt": "2030-05-03T10:00:00+00:00",
            "updatedAt": "2030-05-03T10:00:00+00:00",
            "url": "https://github.com/octo/widgets/issues/5#issuecomment-9001" }),
        "{response}"
    );
    let posts = sent(&forge, "POST", "/issues/5/comments");
    assert_eq!(posts.len(), 1);
    assert_eq!(body(&posts[0]), json!({ "body": "Seen it too." }));
    let pages = forge.heard("/issues/5/comments");
    assert!(
        pages[0].contains("page=2"),
        "101 comments: the last page first"
    );
    let entry = one_entry(&forge).await;
    assert_eq!(entry["action"], "create");
    assert_eq!(entry["change"]["function"], "add_issue_comment");
    assert_eq!(entry["change"]["target"], "issue#5");
    assert_eq!(entry["change"]["created"], 9001);
    asked_again(&forge).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_closing_an_issue_patches_its_state_and_reason(pool: PgPool) {
    let forge = forge(&pool).await;
    listed(&forge).await;
    let path = format!("{}/issues/3", super::super::BASE);
    let stub = &forge.kit.stub;
    stub.reply_once("GET", &path, super::super::ok(&issue(3).to_string()));
    forge.ok("/issues/3", &closed(3, "not_planned"));
    answer(
        &forge,
        ("PATCH", "/issues/3"),
        200,
        &closed(3, "not_planned"),
    );

    let response = forge
        .ask(
            SET_OPEN,
            json!({ "number": 3, "open": false, "reason": "NOT_PLANNED" }),
        )
        .await;

    assert_eq!(
        response["data"]["setIssueOpen"],
        json!({ "number": 3, "open": false, "closeReason": "NOT_PLANNED",
            "closedAt": "2030-05-03T10:00:00+00:00" }),
        "{response}"
    );
    let patches = sent(&forge, "PATCH", "/issues/3");
    assert_eq!(patches.len(), 1);
    assert_eq!(
        body(&patches[0]),
        json!({ "state": "closed", "state_reason": "not_planned" })
    );
    let entry = one_entry(&forge).await;
    assert_eq!(entry["action"], "update");
    assert_eq!(entry["change"]["function"], "set_issue_open");
    assert_eq!(entry["change"]["target"], "issue#3");
    assert_eq!(entry["change"]["open"], false);
    assert_eq!(entry["change"]["reason"], "not_planned");
    asked_again(&forge).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_opening_an_open_issue_sends_nothing_and_records_nothing(pool: PgPool) {
    let forge = forge(&pool).await;
    forge.ok("/issues/3", &issue(3));
    forge.ok("/issues/4", &closed(4, "completed"));

    let open = forge
        .ask(SET_OPEN, json!({ "number": 3, "open": true }))
        .await;
    assert_eq!(open["data"]["setIssueOpen"]["open"], true, "{open}");
    let shut = forge
        .ask(SET_OPEN, json!({ "number": 4, "open": false }))
        .await;
    assert_eq!(shut["data"]["setIssueOpen"]["open"], false, "{shut}");

    assert!(sent(&forge, "PATCH", "/issues/3").is_empty(), "nothing");
    assert!(sent(&forge, "PATCH", "/issues/4").is_empty(), "nothing");
    assert!(audits(&pool).await.is_empty(), "nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_reopening_sends_open_with_no_reason(pool: PgPool) {
    let forge = forge(&pool).await;
    let path = format!("{}/issues/4", super::super::BASE);
    let stub = &forge.kit.stub;
    stub.reply_once(
        "GET",
        &path,
        super::super::ok(&closed(4, "completed").to_string()),
    );
    forge.ok("/issues/4", &issue(4));

    let response = forge
        .ask(SET_OPEN, json!({ "number": 4, "open": true }))
        .await;

    assert_eq!(response["data"]["setIssueOpen"]["open"], true, "{response}");
    let patches = sent(&forge, "PATCH", "/issues/4");
    assert_eq!(body(&patches[0]), json!({ "state": "open" }));
    assert_eq!(audits(&pool).await[0]["change"]["reason"], Value::Null);
}
