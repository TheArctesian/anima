//! A repository's issues and the comments of an issue or a pull request, read live from GitHub
//! (PRJ-031). GitHub lists pull requests as issues; the hub shows them only as pull requests.

mod graphql;
mod views;

pub(super) use self::graphql::Query;
