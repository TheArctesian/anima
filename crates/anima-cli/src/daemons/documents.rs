//! The daemon documents, typed from K1 at build time (CMD-004). `DaemonEnrol` derives `Serialize`
//! but not `Debug`: its answer holds the one-time enrolment code, which only `daemon enrol`
//! prints, on purpose (CMD-014, LOG-007).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/daemons/status.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DaemonStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/daemons/enrol.graphql",
    response_derives = "Serialize",
    variables_derives = "Debug"
)]
pub struct DaemonEnrol;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/daemons/revoke.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DaemonRevoke;
