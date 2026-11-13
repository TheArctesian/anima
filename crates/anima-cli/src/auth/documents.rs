//! The sign-in documents. `Login` derives neither `Debug` nor `Serialize`: its variables hold
//! the password and its answer the token (CMD-014).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/auth/login.graphql",
    skip_serializing_none
)]
pub struct Login;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/auth/logout.graphql",
    response_derives = "Debug"
)]
pub struct Logout;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/auth/me.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct Me;
