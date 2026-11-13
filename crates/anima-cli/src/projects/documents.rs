//! The project documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/projects/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct ProjectList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/projects/get.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct ProjectGet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/projects/leave_out.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct RepositoryLeaveOut;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/projects/restore.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct RepositoryRestore;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/projects/confirm.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct ResourceConfirm;
