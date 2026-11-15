//! A repository's workflows and their runs, read live from GitHub (PRJ-031, PRJ-034,
//! PRJ-035), with the jobs of a run counted when they are selected; and the changes to them:
//! the run controls and a workflow started by hand (OD-E14, PRJ-028).

mod changes;
mod controls;
mod dispatch;
mod graphql;
pub(super) mod jobs;
#[cfg(test)]
mod tests;
mod views;

pub(super) use self::{changes::Mutation, graphql::Query};
