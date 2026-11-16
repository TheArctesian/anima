//! Two extensions meet at a shared subject key without an import (LNK-004).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{link, links_of, signed_in, stored};
use crate::hub::error;

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_two_records_that_name_one_repository_meet_at_its_key(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let repository = "repository:okita/anima.ing";
    link(
        &hub,
        &token,
        ("project:anima", "has_repository", repository),
        Value::Null,
    )
    .await;
    link(
        &hub,
        &token,
        ("assistant_session:42", "worked_on", repository),
        Value::Null,
    )
    .await;
    let listed = links_of(&hub, &token, repository).await;
    let listed = listed["data"]["recordLinks"].as_array().unwrap();
    let mut sources: Vec<_> = listed
        .iter()
        .map(|l| l["source"].as_str().unwrap())
        .collect();
    sources.sort_unstable();
    assert_eq!(sources, ["assistant_session:42", "project:anima"]);
    // A repository key is its own name: it needs no label.
    assert_eq!(listed[0]["targetTitle"], "okita/anima.ing");
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_a_date_key_is_a_date_and_names_itself(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let made = link(
        &hub,
        &token,
        ("note:7", "about", "date:2026-11-16"),
        Value::Null,
    )
    .await;
    assert_eq!(
        made["data"]["linkRecords"]["targetTitle"], "2026-11-16",
        "{made}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_004_a_subject_key_in_a_second_form_is_refused(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    for (target, source) in [
        ("repository:Okita/Anima.ing", "note:7"),
        ("date:2026-11-16T00:00:00Z", "note:7"),
        ("date:16/11/2026", "note:7"),
        ("person:ada", "note:7"),
        ("place:home", "note:7"),
        ("note:7", "person:ada"),
    ] {
        let response = link(&hub, &token, (source, "about", target), json!("label")).await;
        assert_eq!(error(&response).1, "VALIDATION", "{target}: {response}");
    }
    assert_eq!(stored(&pool).await, 0);
}
