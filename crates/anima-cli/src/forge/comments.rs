//! The comments of an issue or a pull request, for `get --comments` (PRJ-030, CMD-026).

use graphql_client::GraphQLQuery;
use serde_json::Value;

use super::render::{many, text};
use crate::connection_page;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::hub::scalars::DateTime;
use crate::output::Table;
use crate::paging::{fetch, page_size};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/forge/comments.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct IssueComments;

connection_page!(issue_comments::IssueCommentsIssueComments);

/// The most comments the hub gives in one page.
const MAX_COMMENTS: i64 = 100;

#[derive(Debug, clap::Args)]
pub struct CommentArgs {
    /// Also show the comments, oldest first.
    #[arg(long)]
    comments: bool,
    /// Comments to show with `--comments`, 1 to 100.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(i64).range(1..=MAX_COMMENTS))]
    comment_limit: i64,
    /// Every comment, not only the first page.
    #[arg(long, requires = "comments")]
    all_comments: bool,
}

/// The issue or pull request whose comments are read.
#[derive(Debug)]
pub struct Thread {
    pub repository: String,
    pub number: i64,
    pub fresh: bool,
}

impl CommentArgs {
    /// `value` with a `comments` connection added when `--comments` is given.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn add_to(
        &self,
        hub: &Hub,
        mut value: Value,
        thread: &Thread,
    ) -> Result<Value, Failure> {
        if !self.comments {
            return Ok(value);
        }
        let first = Some(page_size(
            self.comment_limit,
            self.all_comments,
            MAX_COMMENTS,
        ));
        let Thread {
            repository,
            number,
            fresh,
        } = thread;
        let page = fetch(None, self.all_comments, move |after| async move {
            let variables = issue_comments::Variables {
                repository: repository.clone(),
                number: *number,
                first,
                after,
                fresh: Some(*fresh),
            };
            Ok(hub.run::<IssueComments>(variables).await?.issue_comments)
        })
        .await?;
        value["comments"] = serde_json::to_value(&page).unwrap_or(Value::Null);
        Ok(value)
    }
}

/// The comments of `value`, one line for each, after the fields of a record.
pub fn lines(table: &mut Table, value: &Value) {
    let Some(connection) = value.get("comments") else {
        return;
    };
    for edge in many(connection, "edges") {
        let comment = &edge["node"];
        table.pair(
            "Comment",
            format!(
                "{} {}: {}",
                text(comment, "authorLogin"),
                text(comment, "createdAt"),
                text(comment, "body")
            ),
        );
    }
    if connection["pageInfo"]["hasNextPage"] == Value::Bool(true) {
        table.pair("More", "more comments: add --all-comments");
    }
}
