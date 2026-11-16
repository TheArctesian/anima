//! A link to a record that does not exist keeps its label (LNK-002), and one table holds
//! every link (LNK-001).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{link, links_of, signed_in, stored, unlink};
use crate::hub::error;

const BOOK: &str = "book:isbn-9780141439518";
const PERSON: &str = "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";

#[sqlx::test(migrations = "./migrations")]
async fn lnk_002_a_link_to_a_target_that_does_not_exist_keeps_its_label(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let made = link(
        &hub,
        &token,
        (BOOK, "lent_to", PERSON),
        json!("Ada Lovelace"),
    )
    .await;
    let made = &made["data"]["linkRecords"];
    assert_eq!(made["label"], "Ada Lovelace", "{made}");
    assert_eq!(
        made["targetTitle"],
        Value::Null,
        "no People extension exists"
    );

    // Both ends see the same link, and the label is still there on a later read.
    for record in [BOOK, PERSON] {
        let listed = links_of(&hub, &token, record).await;
        let listed = listed["data"]["recordLinks"].as_array().unwrap();
        assert_eq!(listed.len(), 1, "{record}");
        assert_eq!(listed[0]["id"], made["id"]);
        assert_eq!(
            (
                &listed[0]["source"],
                &listed[0]["kind"],
                &listed[0]["target"]
            ),
            (&json!(BOOK), &json!("lent_to"), &json!(PERSON))
        );
        assert_eq!(listed[0]["label"], "Ada Lovelace");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_001_the_same_link_again_is_one_row_and_a_new_label_replaces_the_old(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let first = link(&hub, &token, (BOOK, "lent_to", PERSON), json!("Ada")).await;
    let again = link(&hub, &token, (BOOK, "lent_to", PERSON), Value::Null).await;
    assert_eq!(again["data"]["linkRecords"], first["data"]["linkRecords"]);
    let renamed = link(
        &hub,
        &token,
        (BOOK, "lent_to", PERSON),
        json!(" Ada Lovelace "),
    )
    .await;
    assert_eq!(
        renamed["data"]["linkRecords"]["id"],
        first["data"]["linkRecords"]["id"]
    );
    assert_eq!(renamed["data"]["linkRecords"]["label"], "Ada Lovelace");
    assert_eq!(stored(&pool).await, 1);

    // Another kind between the same two records is another link.
    link(&hub, &token, (BOOK, "gift_from", PERSON), Value::Null).await;
    assert_eq!(stored(&pool).await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_001_unlink_removes_the_link_once(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let made = link(&hub, &token, (BOOK, "lent_to", PERSON), Value::Null).await;
    let id = &made["data"]["linkRecords"]["id"];
    let removed = unlink(&hub, &token, id).await;
    assert_eq!(&removed["data"]["unlinkRecords"], id, "{removed}");
    assert_eq!(stored(&pool).await, 0);
    for missing in [id.clone(), json!("not-an-id")] {
        let response = unlink(&hub, &token, &missing).await;
        assert_eq!(error(&response).1, "NOT_FOUND", "{response}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_001_input_that_is_not_a_link_is_a_validation_error_and_stores_nothing(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let long = "x".repeat(201);
    for (ends, label, field) in [
        (("book", "lent_to", PERSON), Value::Null, "source"),
        ((BOOK, "lent_to", "Person:1"), Value::Null, "target"),
        ((BOOK, "Lent To", PERSON), Value::Null, "kind"),
        ((BOOK, "lent_to", PERSON), json!(long), "label"),
        ((BOOK, "lent_to", PERSON), json!("two\nlines"), "label"),
        ((BOOK, "same_as", BOOK), Value::Null, "target"),
    ] {
        let response = link(&hub, &token, ends, label).await;
        assert_eq!(error(&response).1, "VALIDATION", "{ends:?}: {response}");
        let fields = &response["errors"][0]["extensions"]["fields"];
        assert_eq!(fields[0]["field"], field, "{ends:?}: {response}");
    }
    assert_eq!(stored(&pool).await, 0);
    let listed = links_of(&hub, &token, "no colon").await;
    assert_eq!(error(&listed).1, "VALIDATION");
}
