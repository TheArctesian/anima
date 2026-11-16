//! The metadata shown by the Projects list.

use chrono::{DateTime, Utc};
use provider_github::Repository;
use sqlx::PgConnection;

use super::FORGE;

fn count(repository: &Repository, field: &str) -> Option<i32> {
    i32::try_from(repository.original.get(field)?.as_u64()?).ok()
}

pub(in crate::work::projects) async fn listed(
    conn: &mut PgConnection,
    repository: &Repository,
) -> Result<(), sqlx::Error> {
    let created: Option<DateTime<Utc>> = repository.original["created_at"]
        .as_str()
        .and_then(|text| text.parse().ok());
    let language = repository.original["language"]
        .as_str()
        .filter(|text| !text.is_empty());
    sqlx::query(
        "UPDATE projects.repositories
         SET repository_created_at = $3, stars = $4, primary_language = $5, size_kib = $6
         WHERE forge = $1 AND external_id = $2 AND left_out_at IS NULL
         AND (repository_created_at, stars, primary_language, size_kib)
             IS DISTINCT FROM ($3, $4, $5, $6)",
    )
    .bind(FORGE)
    .bind(&repository.external_id)
    .bind(created)
    .bind(count(repository, "stargazers_count"))
    .bind(language)
    .bind(count(repository, "size"))
    .execute(conn)
    .await
    .map(drop)
}

pub(in crate::work::projects) async fn history(
    conn: &mut PgConnection,
    (external_id, branch): (&str, &str),
    (last_commit, count): (Option<DateTime<Utc>>, Option<i32>),
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE projects.repositories SET last_commit_at = $4, commit_count = $5
         WHERE forge = $1 AND external_id = $2 AND default_branch = $3 AND left_out_at IS NULL
         AND (last_commit_at, commit_count) IS DISTINCT FROM ($4, $5)",
    )
    .bind(FORGE)
    .bind(external_id)
    .bind(branch)
    .bind(last_commit)
    .bind(count)
    .execute(conn)
    .await?;
    Ok(result.rows_affected() > 0)
}
