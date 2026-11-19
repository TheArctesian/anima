//! The place documents, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct PlaceList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct PlaceShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct PlaceAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct PlaceSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/merge.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct PlaceMerge;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/split.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize",
    skip_serializing_none
)]
pub struct PlaceSplit;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct PlaceRemove;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/places/documents/site.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct PlaceSite;
