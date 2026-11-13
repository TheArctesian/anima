//! The release documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/releases/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct ReleaseList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/releases/next_version.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct ReleaseNextVersion;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/releases/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReleaseCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/releases/cancel.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReleaseCancel;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/releases/log.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct BuildJobLog;
