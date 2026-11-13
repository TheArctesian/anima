//! The documents for the values a new check starts with (K1 `CheckDefaults`).

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/defaults/get.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CheckDefaultsGet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/defaults/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CheckDefaultsSet;
