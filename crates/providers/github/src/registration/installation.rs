//! Reading an installation of the App, or the list of them.

use serde::Deserialize;
use source_exchange::{Installation, Problem};

use crate::problem::problem;

#[derive(Deserialize)]
struct Reply {
    id: u64,
    app_id: u64,
    account: Account,
    suspended_at: Option<String>,
}

#[derive(Deserialize)]
struct Account {
    id: u64,
}

impl From<Reply> for Installation {
    fn from(reply: Reply) -> Self {
        Self {
            id: reply.id,
            app_id: reply.app_id,
            account_id: reply.account.id,
            suspended: reply.suspended_at.is_some(),
        }
    }
}

fn unreadable() -> Problem {
    problem(
        "installation_unreadable",
        "GitHub App installation",
        "GitHub's reply about the App's installation could not be read",
    )
}

pub(crate) fn parse_installation(body: &[u8]) -> Result<Installation, Problem> {
    serde_json::from_slice::<Reply>(body)
        .map(Installation::from)
        .map_err(|_| unreadable())
}

pub(crate) fn parse_installations(body: &[u8]) -> Result<Vec<Installation>, Problem> {
    serde_json::from_slice::<Vec<Reply>>(body)
        .map(|all| all.into_iter().map(Installation::from).collect())
        .map_err(|_| unreadable())
}
