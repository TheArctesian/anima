//! Reports from the lease holder only, redacted and trimmed log lines, the upload report and
//! version refusal (BLD-002, BLD-010, BLD-048, BLD-049, BLD-063, LOG-007).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{claim, create, create_job, data, enrol, operator, report};
use crate::hub::{Hub, error, start};

#[sqlx::test(migrations = "./migrations")]
async fn bld_010_a_builder_without_the_lease_cannot_report(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let holder = enrol(&hub, &operator, "pop-os").await;
    let other = enrol(&hub, &operator, "other").await;
    data(claim(&hub, &holder.token).await);
    let response = report(&hub, &other.token, &job, &["not mine"]).await;
    assert_eq!(
        error(&response),
        (
            "This builder does not hold the job's lease.".to_owned(),
            "FORBIDDEN".to_owned()
        )
    );
    let lines: i64 = sqlx::query_scalar("SELECT count(*) FROM builds.log_lines")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(lines, 0);
    let reported = data(report(&hub, &holder.token, &job, &["mine"]).await);
    assert_eq!(reported["reportBuildJob"]["state"], "BUILDING");
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_049_a_token_in_a_log_line_is_redacted_before_it_is_stored(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    let leaked = format!("curl -H 'Authorization: Bearer {}' https://hub", builder.token);
    let env = format!("GITHUB_TOKEN={} bun run gates", builder.token);
    data(report(&hub, &builder.token, &job, &["Compiling anima", &leaked, &env]).await);

    let stored: Vec<String> =
        sqlx::query_scalar("SELECT text FROM builds.log_lines ORDER BY sequence")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(stored.len(), 3);
    assert_eq!(stored[0], "Compiling anima");
    for line in &stored {
        assert!(!line.contains(&builder.token), "{line}");
    }
    let query = "query($id: ID!) { buildJobLog(id: $id) { sequence text } }";
    let log = hub.post(Some(&operator), query, json!({ "id": job })).await;
    assert!(!log.to_string().contains(&builder.token), "{log}");
    assert_eq!(data(log)["buildJobLog"][2]["sequence"], 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_048_a_job_keeps_the_last_2_mib_of_its_log(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    let line = "x".repeat(4000);
    for _ in 0..3 {
        let lines = vec![line.as_str(); 250];
        data(report(&hub, &builder.token, &job, &lines).await);
    }
    let (bytes, last): (i64, i64) = sqlx::query_as(
        "SELECT sum(octet_length(text))::bigint, max(sequence) FROM builds.log_lines",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(bytes <= 2 * 1024 * 1024, "{bytes}");
    assert!(bytes > 2 * 1024 * 1024 - 4000, "{bytes}");
    assert_eq!(last, 750);
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_063_an_uploaded_report_keeps_the_job_uploading(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let job = create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    data(claim(&hub, &builder.token).await);
    data(report(&hub, &builder.token, &job, &[]).await);
    let uploading = "mutation($id: ID!) { reportBuildJob(id: $id, state: UPLOADING, \
                     logLines: []) { state } }";
    data(hub.post(Some(&builder.token), uploading, json!({ "id": job })).await);
    let query = "mutation($id: ID!, $files: [BuildFileInput!]!, $feed: String!) { \
                 reportBuildUploaded(id: $id, files: $files, feed: $feed) { state } }";
    let files = json!([{ "name": "SHA256SUMS", "sha256": "ab".repeat(32), "size": 90 }]);
    let variables = json!({ "id": job, "files": files, "feed": "{\"version\":\"0.1.12\"}" });
    let uploaded = data(hub.post(Some(&builder.token), query, variables).await);
    assert_eq!(uploaded["reportBuildUploaded"]["state"], "UPLOADING");
    let kept: (String, i64) = sqlx::query_as("SELECT name, size_bytes FROM builds.job_files")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, ("SHA256SUMS".to_owned(), 90));
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_002_a_version_that_is_not_new_is_refused(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    data(create(&hub, &operator, "0.1.12").await);
    for version in ["0.1.12", "0.1", "v0.1.13", "0.01.13"] {
        let response = create(&hub, &operator, version).await;
        assert_eq!(error(&response).1, "VALIDATION", "{version}: {response}");
    }
    let releases: i64 = sqlx::query_scalar("SELECT count(*) FROM builds.releases")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(releases, 1);
    let next = data(hub.post(Some(&operator), "{ nextReleaseVersion }", json!({})).await);
    assert_eq!(next["nextReleaseVersion"], "0.1.13");
}
