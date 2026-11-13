//! The agent documents, typed from K1 at build time (CMD-004). `AgentEnrol` derives `Serialize`
//! but not `Debug`: its answer holds the one-time enrolment code, which only `agent enrol`
//! prints, on purpose (CMD-014, LOG-007).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/agents/status.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct AgentStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/agents/enrol.graphql",
    response_derives = "Serialize",
    variables_derives = "Debug"
)]
pub struct AgentEnrol;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/agents/revoke.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct AgentRevoke;
