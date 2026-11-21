//! The route documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/routes/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RouteList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/routes/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RouteShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/routes/add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RouteAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/routes/check.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RouteCheck;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/routes/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RouteRemove;
