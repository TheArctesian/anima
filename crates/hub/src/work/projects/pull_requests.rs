//! A repository's pull requests, read live from GitHub (PRJ-031, PRJ-033): the list, one pull
//! request, and its mergeability, checks and the repository's merge methods when selected.
//! Merging one, or many one after another, by the merge rule (OD-E14, PRJ-033, PRO-013).

mod back;
mod bulk;
mod changes;
mod graphql;
mod input;
mod merge;
#[cfg(test)]
mod tests;
mod views;

pub(super) use self::{changes::Mutation, graphql::Query};
