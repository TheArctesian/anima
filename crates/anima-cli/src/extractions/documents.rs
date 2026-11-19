//! The extraction proposal documents, typed from K1 at build time (CMD-004). The field inputs
//! also read from JSON, as `--fields` gives them.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extractions/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ExtractionList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extractions/propose.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct ExtractionPropose;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extractions/confirm.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct ExtractionConfirm;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/extractions/reject.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ExtractionReject;
