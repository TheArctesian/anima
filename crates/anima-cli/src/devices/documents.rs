//! The device documents, typed from K1 at build time (CMD-004). `Debug, Serialize` on the
//! answers lets `--json` print them with the K1 field names.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, IpAddr};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct DeviceList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/get.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeviceGet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/sites.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct SiteNames;

// The change documents take their input as JSON built from the flags (`settings.rs`), so
// their input types derive `Deserialize` too.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct DeviceCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/update.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct DeviceUpdate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeviceDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/wake.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeviceWake;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/test.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct DeviceTest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/metrics.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeviceMetrics;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/uptime.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeviceUptime;
