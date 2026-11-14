//! A repository's pull requests, read live from GitHub (PRJ-031, PRJ-033): the list, one pull
//! request, and its mergeability, checks and the repository's merge methods when selected.

mod graphql;
mod views;

pub(super) use self::graphql::Query;
