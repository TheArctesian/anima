//! The event and audit documents, typed from K1 at build time (CMD-004). `Debug, Serialize` on
//! the answers lets `--json` print them with the K1 field names.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, IpAddr};

/// K1 `JSON`: an audit change record, printed as the hub wrote it.
#[expect(
    clippy::upper_case_acronyms,
    reason = "graphql_client looks a scalar up by its K1 name"
)]
type JSON = serde_json::Value;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/events/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct EventList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/events/audit.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct AuditList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/events/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct EventAdded;
