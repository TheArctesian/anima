//! The container documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/containers/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ContainerList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/containers/log.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ContainerLogRead;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/containers/inspect.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ContainerInspect;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/containers/storage.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ContainerStorageRead;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/containers/networks.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ContainerNetworkList;
