//! Every Projects root is `CONFLICT` while Projects is off, and nothing changes (EXT-002);
//! signed out, each is `UNAUTHENTICATED` (API-003).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::github::{Repo, files, list};
use super::kit;
use crate::{
    hub::{error, unauthenticated},
    sources::set_projects,
};

/// Each root with variables that would work while Projects is on.
fn roots(repository: &Value, resource: &Value) -> [(&'static str, Value); 5] {
    [
        ("query { projects { edges { node { id } } } }", Value::Null),
        ("query { project(slug: \"site\") { id } }", Value::Null),
        (
            "mutation($id: ID!) { leaveOutRepository(id: $id) { id } }",
            json!({ "id": repository }),
        ),
        (
            "mutation($id: ID!) { restoreRepository(id: $id) { id } }",
            json!({ "id": repository }),
        ),
        (
            "mutation($id: ID!) { confirmProjectResource(id: $id) { id } }",
            json!({ "id": resource }),
        ),
    ]
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_002_every_root_is_conflict_while_projects_is_off(pool: PgPool) {
    let kit = kit(&pool).await;
    let id = kit.connect().await;
    files(&kit.stub, "me/site", &["Dockerfile"], &[]);
    list(&kit.stub, &[Repo::new(1, "me/site")]);
    kit.run(id).await;
    let site = kit.project("site").await;
    let (repository, resource) = (
        site["repositories"][0]["id"].clone(),
        site["resources"][0]["id"].clone(),
    );

    set_projects(&pool, false).await;
    for (query, variables) in roots(&repository, &resource) {
        let response = kit.post(query, variables).await;
        let (message, code) = error(&response);
        assert_eq!(code, "CONFLICT", "{query}: {response}");
        assert!(message.contains("Projects is off"), "{message}");
        let signed_out = kit.hub.post(None, query, Value::Null).await;
        assert_eq!(error(&signed_out), unauthenticated(), "{query}");
    }
    set_projects(&pool, true).await;
    let site = kit.project("site").await;
    assert_eq!(site["repositories"][0]["leftOut"], false, "nothing changed");
    assert_eq!(site["resources"][0]["state"], "FOUND_IN_REPOSITORY");
}
