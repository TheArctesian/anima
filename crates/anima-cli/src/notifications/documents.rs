//! The notification documents, typed from K1 at build time (CMD-004). A channel's settings can
//! hold a secret (a webhook URL, an SMTP password, an ntfy token), so the documents that send
//! it derive `Deserialize` only (the document is read from a file or standard input) and
//! neither `Debug` nor `Serialize` of their own (CMD-014, LOG-007). K1 returns no secret, so
//! the answers derive `Debug, Serialize` as everywhere else.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::DateTime;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/channel_list.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct ChannelList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/channel_create.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Deserialize",
    skip_serializing_none
)]
pub struct ChannelCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/channel_update.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Deserialize",
    skip_serializing_none
)]
pub struct ChannelUpdate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/channel_delete.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ChannelDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/channel_test.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ChannelTest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/settings_get.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct SettingsGet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/settings_set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SettingsSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/notifications/deliveries.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct DeliveryList;
