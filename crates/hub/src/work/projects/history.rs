//! A repository's history, read live from GitHub (PRJ-030, PRJ-031): its commits with their
//! checks when selected, its branches and its releases.

mod graphql;
mod omitted;
mod views;

pub(super) use self::graphql::Query;
