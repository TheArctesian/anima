//! A repository's code, read live from GitHub (PRJ-033; K-E1 Section 13.8): one directory's
//! names, and one file, which the hub never keeps and shows only to a signed-in operator.

pub(super) mod address;
mod graphql;
mod views;

pub(super) use self::graphql::Query;
