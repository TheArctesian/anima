//! The location documents, typed from K1 at build time (CMD-004). The sample input also reads
//! from JSON, as `anima location import` gives it.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LocationList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/record.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone, PartialEq, Deserialize"
)]
pub struct LocationRecord;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/context.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LocationContext;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/token_add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LocationTokenAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/token_list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LocationTokenList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/locations/token_revoke.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LocationTokenRevoke;
