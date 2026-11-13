//! The trust documents: a host key or certificate fingerprint the operator accepts.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/trust/accept_host_key.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct HostKeyAccept;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/trust/accept_certificate.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CertificateAccept;
