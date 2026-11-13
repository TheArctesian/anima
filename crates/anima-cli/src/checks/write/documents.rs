//! The documents that change a check, typed from K1 at build time (CMD-004). `CheckCreate`
//! and `CheckUpdate` variables can hold an SNMP community, a password or a bearer token, so
//! they derive no `Debug`; `Deserialize` lets a configuration file fill the input.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::IpAddr;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Deserialize",
    skip_serializing_none
)]
pub struct CheckCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/update.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Deserialize",
    skip_serializing_none
)]
pub struct CheckUpdate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/update_ungrouped.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Deserialize",
    skip_serializing_none
)]
pub struct CheckUpdateUngrouped;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CheckDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/pause.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CheckSetPaused;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/write/notifications.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CheckNotificationsSet;
