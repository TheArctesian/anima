//! The audit targets of the sources engine and Projects: stored with their kind and read back
//! (SRC-003, PRO-001).

use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use super::{audit, day, new_audit};
use crate::{
    fleet::events::AuditTarget,
    ids::{ProjectId, SourceAccountId},
};

#[sqlx::test(migrations = "./migrations")]
async fn src_003_audit_entries_for_source_account_and_project_are_stored_and_read_back(
    pool: PgPool,
) {
    let mut conn = pool.acquire().await.unwrap();
    let account = SourceAccountId::from(Uuid::now_v7());
    let project = ProjectId::from(Uuid::now_v7());
    for (target, kind) in [
        (AuditTarget::SourceAccount(account), "source_account"),
        (AuditTarget::Project(project), "project"),
    ] {
        let mut entry = new_audit(serde_json::json!({ "label": "work" }));
        entry.target = target;
        audit(&mut conn, day(2031, 6, 30), entry).await.unwrap();
        let row = sqlx::query("select target_id from events.audit_log where target_type = $1")
            .bind(kind)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(target.kind(), kind);
        assert_eq!(row.get::<&str, _>("target_id"), target.id());
    }
    assert_eq!(AuditTarget::Project(project).id(), project.to_string());
}
