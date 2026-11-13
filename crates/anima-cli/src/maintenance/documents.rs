//! The maintenance documents, typed from K1 at build time (CMD-004). `Deserialize` on the
//! create and update variables lets `fields.rs` fill the input from the flags.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/maintenance/list.graphql",
    response_derives = "Debug, Serialize, Clone",
    variables_derives = "Debug"
)]
pub struct MaintenanceList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/maintenance/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct MaintenanceCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/maintenance/update.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct MaintenanceUpdate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/maintenance/delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct MaintenanceDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/maintenance/active.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct MaintenanceSetActive;
