//! The Connect GitHub documents, typed from K1 at build time (CMD-004, CMD-031). The answer of
//! `GithubConnectOpen` holds the ONE-TIME link, so its types derive no `Debug` and no
//! `Serialize`: nothing can print it by accident (MCP-004, LOG-007).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sources/connect/state.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct GithubConnectState;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sources/connect/start.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct GithubConnectStart;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sources/connect/open.graphql"
)]
pub struct GithubConnectOpen;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sources/connect/cancel.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct GithubConnectCancel;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sources/connect/changed.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct GithubConnectChanged;
