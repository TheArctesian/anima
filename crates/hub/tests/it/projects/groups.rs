//! Project groups (PRO-059 to PRO-064) through GraphQL with a database: make a group with
//! projects, move a project, rename, delete, remove, and the list by group.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list};
use super::{Kit, kit, slugs};
use crate::hub::error;

const GROUP: &str = "id name projectCount createdAt";

/// Imports `me/site`, `me/blog` and `me/docs`; returns the id of each project by slug.
async fn imported(kit: &Kit) -> [String; 3] {
    let id = kit.connect().await;
    for name in ["me/site", "me/blog", "me/docs"] {
        files(&kit.stub, name, &[], &[]);
    }
    list(
        &kit.stub,
        &[
            Repo::new(1, "me/site"),
            Repo::new(2, "me/blog"),
            Repo::new(3, "me/docs"),
        ],
    );
    kit.run(id).await;
    let mut ids = Vec::new();
    for slug in ["site", "blog", "docs"] {
        ids.push(kit.project(slug).await["id"].as_str().unwrap().to_owned());
    }
    ids.try_into().unwrap()
}

async fn create(kit: &Kit, name: &str, projects: &[&String]) -> Value {
    let query = format!(
        "mutation($name: String!, $projects: [ID!]!) {{ createProjectGroup(name: $name, projects: $projects) {{ {GROUP} }} }}"
    );
    kit.post(&query, json!({ "name": name, "projects": projects }))
        .await
}

async fn set(kit: &Kit, projects: &[&String], group: Option<&str>) -> Value {
    kit.post(
        "mutation($projects: [ID!]!, $group: ID) { setProjectsGroup(projects: $projects, group: $group) }",
        json!({ "projects": projects, "group": group }),
    )
    .await
}

async fn groups(kit: &Kit) -> Vec<(String, i64)> {
    let response = kit
        .post(&format!("{{ projectGroups {{ {GROUP} }} }}"), json!({}))
        .await;
    response["data"]["projectGroups"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .iter()
        .map(|g| {
            (
                g["name"].as_str().unwrap().to_owned(),
                g["projectCount"].as_i64().unwrap(),
            )
        })
        .collect()
}

async fn listed(kit: &Kit, browse: Value) -> Vec<String> {
    let response = kit
        .post(
            "query($browse: ProjectBrowse) { projects(browse: $browse, first: 200) { edges { node { slug group { name } } } } }",
            json!({ "browse": browse }),
        )
        .await;
    let edges = response["data"]["projects"]["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"));
    let nodes: Vec<Value> = edges.iter().map(|edge| edge["node"].clone()).collect();
    slugs(&nodes)
}

fn id_of(group: &Value) -> String {
    group["id"].as_str().unwrap().to_owned()
}

async fn audit_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM events.audit_log WHERE target_type = 'project_group'")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_060_a_new_group_holds_the_selected_projects_and_a_move_takes_one_from_its_group(
    pool: PgPool,
) {
    let kit = kit(&pool).await;
    let [site, blog, docs] = imported(&kit).await;
    let work = create(&kit, "  Work ", &[&site, &blog]).await;
    let work = &work["data"]["createProjectGroup"];
    assert_eq!(
        (work["name"].as_str(), work["projectCount"].as_i64()),
        (Some("Work"), Some(2))
    );

    let home = create(&kit, "Home", &[&blog]).await;
    assert_eq!(
        home["data"]["createProjectGroup"]["projectCount"], 1,
        "{home}"
    );
    assert_eq!(
        groups(&kit).await,
        [("Home".to_owned(), 1), ("Work".to_owned(), 1)]
    );
    assert_eq!(kit.project("blog").await["group"]["name"], "Home");

    let moved = set(&kit, &[&docs, &site], Some(&id_of(work))).await;
    assert_eq!(
        moved["data"]["setProjectsGroup"],
        json!([docs]),
        "only docs moved"
    );
    assert_eq!(
        listed(&kit, json!({ "group": id_of(work) })).await,
        ["docs", "site"]
    );
    assert_eq!(
        listed(&kit, json!({ "ungrouped": true })).await,
        Vec::<String>::new()
    );
    assert_eq!(audit_count(&pool).await, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_059_two_groups_never_share_a_name_and_a_bad_name_or_project_is_refused(pool: PgPool) {
    let kit = kit(&pool).await;
    let [site, ..] = imported(&kit).await;
    create(&kit, "Work", &[]).await;
    assert_eq!(error(&create(&kit, "WORK", &[]).await).1, "CONFLICT");
    assert_eq!(error(&create(&kit, "", &[]).await).1, "VALIDATION");
    let unknown = uuid::Uuid::now_v7().to_string();
    assert_eq!(
        error(&create(&kit, "Home", &[&unknown]).await).1,
        "NOT_FOUND"
    );
    assert_eq!(
        groups(&kit).await,
        [("Work".to_owned(), 0)],
        "nothing made by a refusal"
    );

    let home = create(&kit, "Home", &[&site]).await;
    let rename = kit
        .post(
            "mutation($id: ID!) { renameProjectGroup(id: $id, name: \"work\") { name } }",
            json!({ "id": id_of(&home["data"]["createProjectGroup"]) }),
        )
        .await;
    assert_eq!(error(&rename).1, "CONFLICT");
    let rename = kit
        .post(
            "mutation($id: ID!) { renameProjectGroup(id: $id, name: \"Personal\") { name projectCount } }",
            json!({ "id": id_of(&home["data"]["createProjectGroup"]) }),
        )
        .await;
    assert_eq!(
        rename["data"]["renameProjectGroup"],
        json!({ "name": "Personal", "projectCount": 1 })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_062_a_delete_keeps_each_project_and_leaves_none_out(pool: PgPool) {
    let kit = kit(&pool).await;
    let [site, blog, _] = imported(&kit).await;
    let work = create(&kit, "Work", &[&site, &blog]).await;
    let id = id_of(&work["data"]["createProjectGroup"]);
    let deleted = kit
        .post(
            "mutation($id: ID!) { deleteProjectGroup(id: $id) }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(deleted["data"]["deleteProjectGroup"], id);
    assert_eq!(slugs(&kit.projects(false).await).len(), 3);
    assert_eq!(slugs(&kit.projects(true).await), Vec::<String>::new());
    assert!(kit.project("site").await["group"].is_null());
    assert_eq!(
        listed(&kit, json!({ "ungrouped": true })).await,
        ["blog", "docs", "site"]
    );
    assert_eq!(groups(&kit).await, Vec::<(String, i64)>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn pro_063_pro_064_a_removed_project_has_no_group_and_a_left_out_one_keeps_it_uncounted(
    pool: PgPool,
) {
    let kit = kit(&pool).await;
    let [site, blog, docs] = imported(&kit).await;
    create(&kit, "Work", &[&site, &blog, &docs]).await;
    let removed = set(&kit, &[&docs], None).await;
    assert_eq!(removed["data"]["setProjectsGroup"], json!([docs]));
    assert_eq!(listed(&kit, json!({ "ungrouped": true })).await, ["docs"]);

    let repository = kit.project("site").await["repositories"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    kit.leave_out(&repository, true).await;
    assert_eq!(
        groups(&kit).await,
        [("Work".to_owned(), 1)],
        "site is left out, not counted"
    );
    assert_eq!(
        kit.project("site").await["group"]["name"],
        "Work",
        "it keeps its group"
    );

    let both = kit
        .post(
            "{ projects(browse: { group: \"x\", ungrouped: true }) { edges { node { slug } } } }",
            json!({}),
        )
        .await;
    assert_eq!(error(&both).1, "VALIDATION");
}
