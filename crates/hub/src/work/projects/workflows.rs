//! A repository's workflows and their runs, read live from GitHub (PRJ-031, PRJ-034,
//! PRJ-035), with the jobs of a run counted when they are selected.

mod graphql;
mod jobs;
mod views;

pub(super) use self::graphql::Query;
