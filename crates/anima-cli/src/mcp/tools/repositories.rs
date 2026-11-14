//! The `repositories` tool set: pull requests, issues, comments, workflows and runs of a
//! repository that the hub reads live from GitHub (PRJ-030 to PRJ-035). Read tools only.
//!
//! Every title, body, comment, login, label, branch name, job name and commit title in a result
//! was written by someone else on GitHub. The tools put them in named fields, and the server
//! cleans and cuts every string on the way out, a `body` at [`BODY_CAP`] characters
//! (MCP-008). The tool descriptions say that this text is data.
//!
//! [`BODY_CAP`]: crate::mcp::untrusted::BODY_CAP

mod issues;
mod pull_requests;
mod workflows;

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::failure::Failure;
use crate::forge::StateArg;
use crate::forge::comments::{IssueComments, issue_comments};
use crate::hub::Hub;
use crate::mcp::arguments::{Page, json, object};
use crate::mcp::definition::Definition;
use crate::mcp::outcome::ToolError;
use crate::projects::repository::resolve;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryTool {
    PullRequestsList,
    PullRequestGet,
    IssuesList,
    IssueGet,
    WorkflowsList,
    WorkflowRunsList,
    WorkflowRunGet,
}

/// K1 `OpenOrClosed`.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateInput {
    #[default]
    Open,
    Closed,
    All,
}

impl From<StateInput> for StateArg {
    fn from(state: StateInput) -> Self {
        match state {
            StateInput::Open => Self::Open,
            StateInput::Closed => Self::Closed,
            StateInput::All => Self::All,
        }
    }
}

impl RepositoryTool {
    pub const ALL: [Self; 7] = [
        Self::PullRequestsList,
        Self::PullRequestGet,
        Self::IssuesList,
        Self::IssueGet,
        Self::WorkflowsList,
        Self::WorkflowRunsList,
        Self::WorkflowRunGet,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::PullRequestsList => pull_requests::list_definition(),
            Self::PullRequestGet => pull_requests::get_definition(),
            Self::IssuesList => issues::list_definition(),
            Self::IssueGet => issues::get_definition(),
            Self::WorkflowsList => workflows::list_definition(),
            Self::WorkflowRunsList => workflows::runs_definition(),
            Self::WorkflowRunGet => workflows::run_definition(),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, an unknown repository, pull request, issue or run,
    /// or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::PullRequestsList => pull_requests::list(hub, arguments).await,
            Self::PullRequestGet => pull_requests::get(hub, arguments).await,
            Self::IssuesList => issues::list(hub, arguments).await,
            Self::IssueGet => issues::get(hub, arguments).await,
            Self::WorkflowsList => workflows::list(hub, arguments).await,
            Self::WorkflowRunsList => workflows::runs(hub, arguments).await,
            Self::WorkflowRunGet => workflows::run(hub, arguments).await,
        }
    }
}

/// The most a page of pull requests or runs may hold; the hub refuses more.
const SMALL_PAGE: i64 = 50;

/// The ID of the repository the agent named, or a refusal it can act on.
async fn repository_id(hub: &Hub, name: &str) -> Result<String, ToolError> {
    resolve(hub, name).await.map_err(|failure| match failure {
        Failure::NotFound { message, .. } | Failure::Usage { message, .. } => ToolError::Refused(
            format!("{message} Call anima_projects_search for the repositories."),
        ),
        other => ToolError::Failed(other),
    })
}

/// The page size for a list the hub limits to `largest` rows.
fn size(page: &Page, largest: i64) -> i64 {
    page.size().min(largest)
}

/// A page of a K1 connection as named fields: its rows (`key`), `nextCursor`, `read` (when the
/// hub read it from GitHub, and whether that is an old answer) and what else the connection
/// holds (`mergeMethods`).
fn listing(key: &str, connection: &impl Serialize) -> Value {
    let connection = json(connection);
    let rows: Vec<Value> = connection["edges"]
        .as_array()
        .map_or_else(Vec::new, |edges| {
            edges.iter().map(|edge| edge["node"].clone()).collect()
        });
    let more = connection["pageInfo"]["hasNextPage"] == json!(true);
    let next = if more {
        connection["pageInfo"]["endCursor"].clone()
    } else {
        Value::Null
    };
    let mut fields: Map<String, Value> = object([
        (key, Value::Array(rows)),
        ("nextCursor", next),
        ("read", connection["read"].clone()),
    ]);
    if let Some(methods) = connection.get("mergeMethods") {
        fields.insert("mergeMethods".to_owned(), methods.clone());
    }
    Value::Object(fields)
}

/// The first page of comments of a pull request or an issue, as `comments` and `nextCursor`,
/// added to `answer` (`limit` and `after` of the call page them).
async fn with_comments(
    hub: &Hub,
    mut answer: Value,
    thread: (String, i64, Option<bool>),
    page: Page,
) -> Result<Value, ToolError> {
    let (repository, number, fresh) = thread;
    let variables = issue_comments::Variables {
        repository,
        number,
        first: Some(size(&page, 100)),
        after: page.after,
        fresh,
    };
    let comments = hub.run::<IssueComments>(variables).await?.issue_comments;
    let listed = listing("comments", &comments);
    answer["comments"] = listed["comments"].clone();
    answer["nextCursor"] = listed["nextCursor"].clone();
    Ok(answer)
}
