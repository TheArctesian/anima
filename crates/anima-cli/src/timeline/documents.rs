//! The timeline document, typed from K1 at build time (CMD-004).

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/timeline/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Clone",
    skip_serializing_none
)]
pub struct TimelineList;
