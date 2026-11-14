//! The pull request documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/pull_requests/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct PullRequestList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/pull_requests/get.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct PullRequestGet;
