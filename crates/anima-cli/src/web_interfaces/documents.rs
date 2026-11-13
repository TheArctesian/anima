//! The web interface documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/current.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebCurrent;

// The input is built from flags as JSON (`fields.rs`), so its type derives `Deserialize` too.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct WebCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/update.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct WebUpdate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/accept_suggestion.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebSuggestionAccept;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/dismiss_suggestion.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebSuggestionDismiss;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/web_interfaces/accept_certificate.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct WebCertificateAccept;
