//! The sealed-record documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sealed_records/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SealedRecordShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sealed_records/store.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SealedRecordStore;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/sealed_records/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SealedRecordRemove;
