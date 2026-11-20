//! The reminder documents, typed from K1 at build time (CMD-004). A trigger and `unless` are
//! JSON in the stored form of K-E6 Section 2.

use graphql_client::GraphQLQuery;

use crate::hub::scalars::{DateTime, JSON};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/list.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderList;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/show.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderShow;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/add.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct ReminderAdd;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/set.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug, Deserialize"
)]
pub struct ReminderSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/remove.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderRemove;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/enable.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderEnable;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/test.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderTest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/firings.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderFirings;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/snooze.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderSnooze;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/ack.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderAck;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/reminders/documents/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct ReminderWatch;
