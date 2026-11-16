use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    Kit,
    github::{Repo, files, list},
    kit,
};
use crate::sources::stub::{Reply, Stub};

fn history(stub: &Stub, repo: &str, at: &str, commits: i32) {
    stub.reply(
        &format!("/repos/{repo}/commits?sha=main&per_page=1"),
        Reply {
            status: 200,
            body: json!([{ "commit": { "committer": { "date": at } } }]).to_string(),
            headers: vec![("link", format!("<https://api.github.com/repos/{repo}/commits?per_page=1&page={commits}>; rel=\"last\""))],
        },
    );
}

async fn page(kit: &Kit, browse: Value, after: Option<&str>) -> Value {
    let result = kit
        .post(
            "query($browse: ProjectBrowse, $after: String) {
           projects(browse: $browse, first: 2, after: $after) {
             languages
             edges { node { slug } }
             pageInfo { hasNextPage endCursor }
           }
         }",
            json!({ "browse": browse, "after": after }),
        )
        .await;
    assert!(result["errors"].is_null(), "{result}");
    result["data"]["projects"].clone()
}

fn names(page: &Value) -> Vec<&str> {
    page["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["slug"].as_str().unwrap())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_001_sorts_and_filters_synced_metadata_before_paging(pool: PgPool) {
    let kit = kit(&pool).await;
    let account = kit.connect().await;
    let mut a = Repo::new(1, "me/alpha");
    a.language = Some("Rust");
    a.size_kib = 800;
    a.stars = 5;
    let mut b = Repo::new(2, "me/beta");
    b.language = Some("TypeScript");
    b.size_kib = 3000;
    b.stars = 20;
    let mut c = Repo::new(3, "me/charlie");
    c.language = Some("Rust");
    c.size_kib = 3000;
    c.stars = 20;
    c.created_at = "2030-03-01T10:00:00Z";
    for repo in [&a, &b, &c] {
        files(&kit.stub, &repo.full_name, &[], &[]);
    }
    history(&kit.stub, &a.full_name, "2030-01-01T10:00:00Z", 7);
    history(&kit.stub, &b.full_name, "2030-03-01T10:00:00Z", 10);
    history(&kit.stub, &c.full_name, "2030-02-01T10:00:00Z", 200);
    list(&kit.stub, &[a, b, c]);
    kit.run(account).await;

    let stars = json!({ "order": "STARS" });
    let first = page(&kit, stars.clone(), None).await;
    assert_eq!(names(&first), ["beta", "charlie"]);
    assert_eq!(first["pageInfo"]["hasNextPage"], true);
    assert_eq!(
        names(&page(&kit, stars, first["pageInfo"]["endCursor"].as_str()).await),
        ["alpha"]
    );
    assert_eq!(
        names(&page(&kit, json!({ "order": "LAST_COMMIT" }), None).await),
        ["beta", "charlie"]
    );
    assert_eq!(
        names(&page(&kit, json!({ "order": "COMMIT_COUNT" }), None).await),
        ["charlie", "beta"]
    );
    assert_eq!(
        names(&page(&kit, json!({ "order": "CREATED" }), None).await),
        ["charlie", "alpha"]
    );
    let filtered = page(
        &kit,
        json!({ "language": "Rust", "minSizeKib": 1000, "maxSizeKib": 4000 }),
        None,
    )
    .await;
    assert_eq!(names(&filtered), ["charlie"]);
    assert_eq!(filtered["languages"], json!(["Rust", "TypeScript"]));
}
