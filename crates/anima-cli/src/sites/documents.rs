//! The site documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::Cidr;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sites/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SiteList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sites/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SiteCreate;

// `deviceId` is nullable here: no `skip_serializing_none`, so a cleared relay sends `null`.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sites/set_wake_relay.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SiteSetWakeRelay;
