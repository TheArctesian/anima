//! The retention documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/retention/list.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct RetentionList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/retention/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RetentionSet;
