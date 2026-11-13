//! The builder documents, typed from K1 at build time (CMD-004). `BuilderEnrol` derives neither
//! `Debug` nor `Serialize`: its answer holds the builder token (CMD-014).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, IpAddr};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/builders/list.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct BuilderList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/builders/enrol.graphql",
    variables_derives = "Debug"
)]
pub struct BuilderEnrol;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/builders/revoke.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct BuilderRevoke;
