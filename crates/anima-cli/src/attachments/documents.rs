//! The file and archive documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/attachments/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct AttachmentList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/attachments/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct AttachmentRemove;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/attachments/archive_add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ArchiveAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/attachments/archive_remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ArchiveRemove;
