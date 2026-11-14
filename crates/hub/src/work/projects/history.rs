//! A repository's history, read live from GitHub (PRJ-030, PRJ-031): its commits with their
//! checks when selected, its branches and its releases; and a commit status Anima writes
//! (OD-E14, PRO-010).

mod changes;
mod graphql;
mod omitted;
mod views;

pub(super) use self::{changes::Mutation, graphql::Query};
