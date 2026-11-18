//! Archives are the unit of deletion (FIL-004), and a file's piece goes with its last
//! attachment (FIL-001).

use std::sync::{Arc, Mutex};

use anima_hub::{
    engine::files::{Derived, Unavailable},
    ids::ArchiveId,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};

use super::{BOOK, FileHub, count};

/// A domain that made two records from each archive, and notes each archive it was asked about.
#[derive(Default)]
struct Notes {
    asked: Mutex<Vec<ArchiveId>>,
}

#[async_trait]
impl Derived for Notes {
    fn name(&self) -> &'static str {
        "notes"
    }

    async fn delete_from_archive(
        &self,
        _conn: &mut PgConnection,
        archive: ArchiveId,
    ) -> Result<u64, Unavailable> {
        self.asked.lock().unwrap().push(archive);
        Ok(2)
    }
}

async fn create_archive(hub: &FileHub) -> String {
    let made = hub
        .post(
            "mutation($owner: String!, $name: String!) { createArchive(owner: $owner, name: $name) \
             { id owner name } }",
            json!({ "owner": BOOK, "name": "Takeout 2026-11" }),
        )
        .await;
    made["data"]["createArchive"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_004_deleting_an_archive_deletes_its_files_its_records_and_unused_pieces(pool: PgPool) {
    let notes = Arc::new(Notes::default());
    let hub = FileHub::deriving(&pool, vec![notes.clone()]).await;
    let archive = create_archive(&hub).await;
    let in_archive = format!("owner={BOOK}&name=a.json&key=engine&archive={archive}");
    hub.upload(&in_archive, b"only in the archive").await;
    hub.upload(&in_archive, b"also kept outside").await;
    let outside = hub
        .upload(
            &format!("owner={BOOK}&name=b.json&key=engine"),
            b"also kept outside",
        )
        .await;

    let deleted = hub
        .post(
            "mutation($id: ID!) { deleteArchive(id: $id) }",
            json!({ "id": archive }),
        )
        .await;

    assert_eq!(
        deleted["data"]["deleteArchive"],
        archive.as_str(),
        "{deleted}"
    );
    assert_eq!(notes.asked.lock().unwrap().len(), 1);
    assert_eq!(notes.asked.lock().unwrap()[0].to_string(), archive);
    assert_eq!(count(&pool, "archive").await, 0);
    assert_eq!(count(&pool, "attachment").await, 1);
    assert_eq!(count(&pool, "piece").await, 1, "the piece outside stays");
    let pieces = hub.stored_files();
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    assert!(pieces[0].ends_with(outside["hash"].as_str().unwrap()));
    let again = hub
        .post(
            "mutation($id: ID!) { deleteArchive(id: $id) }",
            json!({ "id": archive }),
        )
        .await;
    assert_eq!(again["errors"][0]["extensions"]["code"], "NOT_FOUND");
    let audit: i64 = sqlx::query_scalar(
        "select count(*) from events.audit_log where target_type in ('archive', 'attachment') \
         and action = 'delete'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        audit, 3,
        "one for the archive and one for each of its files"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_001_deleting_an_attachment_removes_its_piece_only_when_no_other_uses_it(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let query = format!("owner={BOOK}&name=a.txt&key=engine");
    let first = hub.upload(&query, b"shared bytes").await;
    let second = hub.upload(&query, b"shared bytes").await;
    let remove = |id: &Value| {
        let id = id.as_str().unwrap().to_owned();
        let hub = &hub;
        async move {
            hub.post(
                "mutation($id: ID!) { deleteAttachment(id: $id) }",
                json!({ "id": id }),
            )
            .await
        }
    };

    remove(&first["id"]).await;
    assert_eq!(
        (count(&pool, "piece").await, hub.stored_files().len()),
        (1, 1)
    );
    let (status, _, sent) = hub.download(second["id"].as_str().unwrap()).await;
    assert_eq!(
        (status.as_u16(), sent.as_slice()),
        (200, b"shared bytes".as_slice())
    );

    remove(&second["id"]).await;
    assert_eq!(
        (count(&pool, "piece").await, hub.stored_files().len()),
        (0, 0)
    );
    let gone = remove(&second["id"]).await;
    assert_eq!(gone["errors"][0]["extensions"]["code"], "NOT_FOUND");
}
