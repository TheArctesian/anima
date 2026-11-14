//! Leave out and restore (PRO-002): a left-out repository is never imported or read again,
//! even renamed; a restore brings its proposals back and its files are read on the next sync.
//! Each change leaves one audit entry with the actor kind.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list, reads_of, tree_reads};
use super::{Kit, by_slug, kit, slugs};
use crate::{hub::error, sources::last_run};

/// Imports `me/site` (with a Dockerfile) and `me/blog`; returns the account and site's
/// repository id.
async fn imported(kit: &Kit) -> (anima_hub::ids::SourceAccountId, String) {
    let id = kit.connect().await;
    files(&kit.stub, "me/site", &["Dockerfile"], &[]);
    files(&kit.stub, "me/blog", &[], &[]);
    list(
        &kit.stub,
        &[Repo::new(1, "me/site"), Repo::new(2, "me/blog")],
    );
    kit.run(id).await;
    let site = kit.project("site").await;
    let repository = site["repositories"][0]["id"].as_str().unwrap().to_owned();
    (id, repository)
}

fn kinds(project: &Value) -> Vec<(String, String)> {
    let resources = project["resources"].as_array().unwrap();
    resources
        .iter()
        .map(|r| {
            let kind = r["kind"].as_str().unwrap().to_owned();
            (kind, r["state"].as_str().unwrap().to_owned())
        })
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_002_a_left_out_repository_stays_out_and_its_files_are_never_read(pool: PgPool) {
    let kit = kit(&pool).await;
    let (id, repository) = imported(&kit).await;
    assert_eq!(tree_reads(&kit.stub, "me/site"), 1);

    let response = kit.leave_out(&repository, true).await;
    let project = &response["data"]["leaveOutRepository"];
    assert_eq!(project["repositories"][0]["leftOut"], true, "{response}");
    assert_eq!(
        kinds(project),
        Vec::<(String, String)>::new(),
        "proposal withdrawn"
    );
    assert_eq!(slugs(&kit.projects(false).await), ["blog"]);
    assert_eq!(slugs(&kit.projects(true).await), ["site"]);

    // A push upstream would queue its files to be read; the hub must not read them.
    let reads = reads_of(&kit.stub, "me/site");
    let mut pushed = Repo::new(1, "me/site");
    pushed.pushed_at = "2030-05-03T10:00:00Z".to_owned();
    list(&kit.stub, &[pushed.clone(), Repo::new(2, "me/blog")]);
    kit.run(id).await;
    pushed.pushed_at = "2030-05-04T10:00:00Z".to_owned();
    list(&kit.stub, &[pushed, Repo::new(2, "me/blog")]);
    kit.run(id).await;
    assert_eq!(
        reads_of(&kit.stub, "me/site"),
        reads,
        "no tree or deployments request"
    );
    assert_eq!(slugs(&kit.projects(false).await), ["blog"]);
    assert_eq!(slugs(&kit.projects(true).await), ["site"]);
    let (_, outcome, inserted, ..) = last_run(&pool, id).await;
    assert_eq!((outcome.as_deref(), inserted), (Some("ok"), 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_002_a_left_out_repository_renamed_upstream_is_not_imported_again(pool: PgPool) {
    let kit = kit(&pool).await;
    let (id, repository) = imported(&kit).await;
    kit.leave_out(&repository, true).await;
    list(
        &kit.stub,
        &[Repo::new(1, "me/new-site"), Repo::new(2, "me/blog")],
    );
    kit.run(id).await;
    assert_eq!(last_run(&pool, id).await.2, 0, "nothing inserted");
    assert_eq!(slugs(&kit.projects(false).await), ["blog"]);
    let left = kit.projects(true).await;
    assert_eq!(slugs(&left), ["site"]);
    assert_eq!(left[0]["repositories"][0]["id"], repository.as_str());
    assert_eq!(reads_of(&kit.stub, "me/new-site"), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_002_restore_brings_the_proposals_back_and_reads_the_files_again(pool: PgPool) {
    let kit = kit(&pool).await;
    let (id, repository) = imported(&kit).await;
    kit.leave_out(&repository, true).await;
    kit.run(id).await;
    // While it is left out the repository gains a desktop app.
    files(
        &kit.stub,
        "me/site",
        &["Dockerfile", "apps/x/src-tauri/tauri.conf.json"],
        &[],
    );
    let trees = tree_reads(&kit.stub, "me/site");

    let response = kit.leave_out(&repository, false).await;
    let project = &response["data"]["restoreRepository"];
    assert_eq!(project["repositories"][0]["leftOut"], false, "{response}");
    assert_eq!(
        kinds(project),
        [("SERVER".to_owned(), "FOUND_IN_REPOSITORY".to_owned())]
    );
    let due: bool =
        sqlx::query_scalar("SELECT next_sync_at <= now() FROM sources.accounts WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(due, "the restore makes the account due");

    kit.run(id).await;
    assert_eq!(tree_reads(&kit.stub, "me/site"), trees + 1, "read again");
    let site = kit.project("site").await;
    let mut found = kinds(&site);
    found.sort();
    assert_eq!(
        found,
        [
            (
                "DESKTOP_APPLICATION".to_owned(),
                "FOUND_IN_REPOSITORY".to_owned()
            ),
            ("SERVER".to_owned(), "FOUND_IN_REPOSITORY".to_owned()),
        ]
    );
    assert_eq!(slugs(&kit.projects(false).await), ["blog", "site"]);
}

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(t) FROM events.audit_log t WHERE target_type = 'project' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_004_leave_out_and_restore_each_leave_one_audit_entry_and_a_repeat_none(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, repository) = imported(&kit).await;
    let site = kit.project("site").await;
    kit.leave_out(&repository, true).await;
    kit.leave_out(&repository, true).await;
    kit.leave_out(&repository, false).await;
    let entries = audits(&pool).await;
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0]["action"], "deactivate");
    assert_eq!(entries[1]["action"], "activate");
    for entry in &entries {
        assert_eq!(entry["target_id"], site["id"]);
        assert_eq!(entry["actor_kind"], "user");
        assert_eq!(entry["change"]["repository"], repository.as_str());
        let text = entry.to_string();
        assert!(!text.contains("Ignore previous"), "no GitHub text: {text}");
    }
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE message LIKE '%project site%' ORDER BY at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(events.len(), 2, "{events:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn an_unknown_or_malformed_repository_id_is_not_found_or_invalid(pool: PgPool) {
    let kit = kit(&pool).await;
    imported(&kit).await;
    let unknown = kit
        .leave_out("0190d6a4-0000-7000-8000-000000000000", true)
        .await;
    assert_eq!(error(&unknown).1, "NOT_FOUND", "{unknown}");
    let malformed = kit.leave_out("me/site", true).await;
    assert_eq!(error(&malformed).1, "VALIDATION", "{malformed}");
    let site = by_slug(&kit.projects(false).await, "site").clone();
    assert_eq!(site["repositories"][0]["leftOut"], json!(false));
}
