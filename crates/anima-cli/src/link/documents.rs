//! The link documents, typed from K1 at build time (CMD-004). `LinkInvite` derives neither
//! `Debug` nor `Serialize`: its answer holds an invite (CMD-014).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, IpAddr};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/link/devices.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct LinkDevices;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/link/invite.graphql"
)]
pub struct LinkInvite;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/link/revoke.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct LinkRevoke;
