//! The VPN documents, typed from K1 at build time (CMD-004). `VpnInvite` and `VpnRouterKey`
//! derive neither `Debug` nor `Serialize`: their answers hold a pre-auth key (CMD-014).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{Cidr, DateTime, IpAddr};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/status.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct VpnStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/invite.graphql"
)]
pub struct VpnInvite;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/router_key.graphql",
    variables_derives = "Debug"
)]
pub struct VpnRouterKey;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/register.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct VpnRegister;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/renew.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct VpnRenew;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/approve_route.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct VpnApproveRoute;
