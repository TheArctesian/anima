//! The record-link documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/record_links/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RecordLinkList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/record_links/add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct RecordLinkAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/record_links/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RecordLinkRemove;
