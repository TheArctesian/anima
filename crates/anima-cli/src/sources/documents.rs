//! The source documents, typed from K1 at build time (CMD-004). The variables of `SourceConnect`
//! hold the GitHub App's private key, so they derive no `Debug` (the generated code adds
//! `Serialize` only, to send them) and the caller wipes them after the request (CMD-014,
//! PRJ-018, LOG-007).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/kinds.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct SourceKinds;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/accounts.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct SourceAccounts;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/connect.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct SourceConnect;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/sync.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SourceSync;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/pause.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SourcePause;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SourceRemove;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sources/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct SourceAccountChanged;
