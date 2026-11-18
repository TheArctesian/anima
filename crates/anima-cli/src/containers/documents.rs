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
