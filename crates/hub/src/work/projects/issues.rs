//! A repository's issues and the comments of an issue or a pull request, read live from GitHub
//! (PRJ-031). GitHub lists pull requests as issues; the hub shows them only as pull requests.
//! The first changes Anima makes on GitHub: make an issue, comment, close or open one
//! (OD-E14, PRJ-028).

mod back;
mod changes;
mod graphql;
#[cfg(test)]
mod tests;
mod views;

pub(super) use self::{changes::Mutation, graphql::Query};
