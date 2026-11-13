//! The session documents, typed from K1 at build time (CMD-004). `SessionStart` and
//! `SessionOpenFiles` derive neither `Debug` nor `Serialize`: their answers hold a ticket token
//! (CMD-014, API-009).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, IpAddr};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sessions/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct SessionList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sessions/end.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SessionEnd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sessions/start.graphql",
    skip_serializing_none
)]
pub struct SessionStart;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sessions/open_files.graphql"
)]
pub struct SessionOpenFiles;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/sessions/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SessionChanged;
