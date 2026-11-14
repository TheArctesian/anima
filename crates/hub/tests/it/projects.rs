//! Projects end to end: the real Projects intake fed by the sources engine from the loopback
//! GitHub stub, read and changed through GraphQL (XP-02; PRO-001 to PRO-004, PRO-006,
//! PRJ-016, SRC-012, EXT-002).

mod accounts;
mod github;
mod guard;
mod import;
mod leave_out;
mod resources;

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    engine::sources::{self, Outcome, Reach, Registry, Scheduling, Transport, Trigger},
    fleet::{events::Bus, targets::AddressPolicy},
    ids::SourceAccountId,
    schema::Dependencies,
    work::projects,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::{
    hub::{Hub, dependencies, set_operator, start, test_vault},
    sources::{CONNECT, TEST_KEY, input, set_projects, stub::Stub},
};

/// Every field of a project the client and the command line read.
pub const PROJECT: &str = "id slug name subjectKey syncedAt \
    repositories { id fullName url defaultBranch private archived fork description pushedAt leftOut gone } \
    resources { id kind state evidence { sign path } foundAt confirmedAt }";

/// A signed-in hub whose sources reach the stub only, with the Projects intake wired in as
/// `main.rs` wires it.
pub struct Kit {
    pub pool: PgPool,
    pub hub: Hub,
    pub token: String,
    pub stub: Stub,
    reach: Reach,
}

impl Kit {
    /// Runs one sync of `id` now, as "Sync now" would, and waits for its end.
    pub async fn run(&self, id: SourceAccountId) -> Option<Outcome> {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let scheduling = Scheduling::new(
            (self.pool.clone(), Bus::new(16), clock),
            test_vault(),
            self.reach.clone(),
        );
        sources::run(&scheduling, id, Trigger::Manual)
            .await
            .unwrap()
    }

    /// Connects the test App with Projects off, so no run starts on its own; then sets
    /// Projects on. Returns the account id.
    pub async fn connect(&self) -> SourceAccountId {
        self.connect_installation("77").await
    }

    /// As [`Kit::connect`], for the installation `installation` of the test App: a second
    /// installation is a second account that lists the same repositories from the stub.
    pub async fn connect_installation(&self, installation: &str) -> SourceAccountId {
        let mut variables = input(&TEST_KEY);
        variables["input"]["installationId"] = json!(installation);
        let response = self.hub.post(Some(&self.token), CONNECT, variables).await;
        let id = response["data"]["connectGithubApp"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("connect failed: {response}"))
            .parse()
            .unwrap();
        set_projects(&self.pool, true).await;
        id
    }

    /// Posts `query` as the signed-in operator.
    pub async fn post(&self, query: &str, variables: Value) -> Value {
        self.hub.post(Some(&self.token), query, variables).await
    }

    /// `projects(leftOut: left_out)`: every node of the first page.
    pub async fn projects(&self, left_out: bool) -> Vec<Value> {
        let query = format!(
            "query($leftOut: Boolean) {{ projects(leftOut: $leftOut, first: 200) \
             {{ edges {{ node {{ {PROJECT} }} }} pageInfo {{ hasNextPage }} }} }}"
        );
        let response = self.post(&query, json!({ "leftOut": left_out })).await;
        let edges = response["data"]["projects"]["edges"]
            .as_array()
            .unwrap_or_else(|| panic!("no projects: {response}"));
        edges.iter().map(|edge| edge["node"].clone()).collect()
    }

    /// The project with this slug, or null.
    pub async fn project(&self, slug: &str) -> Value {
        let query = format!("query($slug: String!) {{ project(slug: $slug) {{ {PROJECT} }} }}");
        let response = self.post(&query, json!({ "slug": slug })).await;
        assert!(response["errors"].is_null(), "{response}");
        response["data"]["project"].clone()
    }

    /// Leaves out (`true`) or restores the repository with this id; the whole response.
    pub async fn leave_out(&self, repository: &str, left_out: bool) -> Value {
        let field = if left_out {
            "leaveOutRepository"
        } else {
            "restoreRepository"
        };
        let query = format!("mutation($id: ID!) {{ {field}(id: $id) {{ {PROJECT} }} }}");
        self.post(&query, json!({ "id": repository })).await
    }
}

/// A signed-in hub with Projects off; its sources reach a new stub.
pub async fn kit(pool: &PgPool) -> Kit {
    let stub = Stub::start(pool).await;
    let transport = Transport::to_stub(
        stub.base.clone(),
        AddressPolicy::default().allowing_loopback(),
    )
    .unwrap();
    set_operator(pool).await;
    let registry = Registry::new(vec![provider_github::DECLARATION], vec![projects::intake()]);
    let reach = Reach::new(registry, transport);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let parts = Dependencies {
        sources: reach.clone(),
        ..dependencies(pool, clock, Bus::new(16))
    };
    let hub = Hub::serving(pool, parts);
    let token = hub.token().await;
    Kit {
        pool: pool.clone(),
        hub,
        token,
        stub,
        reach,
    }
}

/// The node with this slug among `projects`.
pub fn by_slug<'a>(projects: &'a [Value], slug: &str) -> &'a Value {
    projects
        .iter()
        .find(|project| project["slug"] == slug)
        .unwrap_or_else(|| panic!("no project {slug}: {projects:?}"))
}

/// The slugs of `projects`, in their order.
pub fn slugs(projects: &[Value]) -> Vec<String> {
    projects
        .iter()
        .map(|project| project["slug"].as_str().unwrap().to_owned())
        .collect()
}
