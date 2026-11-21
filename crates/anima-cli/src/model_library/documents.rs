//! The model library documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/alias.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelAlias;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/sources.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelSources;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/add_source.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelSourceAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/approval.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelApproval;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/model_library/set_approval.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ModelSetApproval;
