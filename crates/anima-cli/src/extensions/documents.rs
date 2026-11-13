//! The extension documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extensions/list.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct ExtensionList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extensions/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ExtensionSet;
