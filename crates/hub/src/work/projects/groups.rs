//! Project groups (PRO-059 to PRO-064): the operator's own labels, one for each project at most.
//! The rules are pure; the store is the only SQL of the table of groups; the GraphQL part gives
//! the query `projectGroups` and the four mutations of K1.

pub(super) mod graphql;
mod rules;
pub(super) mod store;

pub(super) use self::graphql::GroupView;
