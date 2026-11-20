//! The project group documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/groups.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectGroups;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectGroupCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/rename.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectGroupRename;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectGroupDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectsGroupSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/projects/group/project_id.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ProjectIdBySlug;
