//! The agent usage document, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/agents/usage.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct AgentUsage;
