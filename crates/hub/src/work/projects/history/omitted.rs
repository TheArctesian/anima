//! What K1 shows of a commit list and a release list that the provider's types leave out: the
//! shas of each commit's parents and the time each release was made. Read from the same body,
//! in the same order, as `issues` reads the pull request mark. Pure.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Body, Problem, Response};

#[derive(Deserialize)]
struct Parent {
    sha: String,
}

#[derive(Deserialize)]
struct Lineage {
    parents: Vec<Parent>,
}

#[derive(Deserialize)]
struct Made {
    created_at: DateTime<Utc>,
}

/// Fixed words: never a piece of GitHub's reply (LOG-007).
fn unreadable() -> Problem {
    Problem {
        code: "reply_unreadable",
        subject: "history".to_owned(),
        message: "GitHub's reply could not be read".to_owned(),
    }
}

fn list<T: for<'de> Deserialize<'de>>(response: &Response<'_>) -> Result<Vec<T>, Problem> {
    match response.body {
        Body::Read(body) => serde_json::from_slice(body).map_err(|_| unreadable()),
        Body::TooLarge => Err(unreadable()),
    }
}

/// The parents' shas of each commit of a list, in the list's order.
///
/// # Errors
/// A [`Problem`] when the body is not a list of commits with parents.
pub fn parents(response: &Response<'_>) -> Result<Vec<Vec<String>>, Problem> {
    Ok(list::<Lineage>(response)?
        .into_iter()
        .map(|commit| {
            commit
                .parents
                .into_iter()
                .map(|parent| parent.sha)
                .collect()
        })
        .collect())
}

/// When each release of a list was made, in the list's order.
///
/// # Errors
/// A [`Problem`] when the body is not a list of releases with that time.
pub fn made(response: &Response<'_>) -> Result<Vec<DateTime<Utc>>, Problem> {
    Ok(list::<Made>(response)?
        .into_iter()
        .map(|release| release.created_at)
        .collect())
}
