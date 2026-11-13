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
