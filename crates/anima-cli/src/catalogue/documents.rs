//! The catalogue documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/catalogue/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CatalogueList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/catalogue/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CatalogueShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/catalogue/install.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct CatalogueInstallAsk;
