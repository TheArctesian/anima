//! The source account an operator names: by ID or by exact label.

use super::documents::{SourceAccounts, source_accounts};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;

/// The two things a command needs of an account it only acts on.
#[derive(Debug, PartialEq, Eq)]
pub struct Named {
    pub id: String,
    pub label: String,
}

/// The account `name` is the ID or the exact label of.
///
/// # Errors
/// [`Failure::NotFound`] when none has it; [`Failure::Usage`] when more than one has the label;
/// the hub's refusal.
pub async fn resolve(hub: &Hub, name: &str) -> Result<Named, Failure> {
    let variables = source_accounts::Variables {
        source_key: None,
        runs: Some(1),
    };
    let accounts = hub.run::<SourceAccounts>(variables).await?.source_accounts;
    let mut matches: Vec<Named> = accounts
        .into_iter()
        .filter(|account| account.id == name || account.label == name)
        .map(|account| Named {
            id: account.id,
            label: account.label,
        })
        .collect();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Failure::NotFound {
            message: format!("There is no source account with the ID or label {name}."),
            hint: Some(Hint::run("anima source list")),
        }),
        _ => Err(Failure::Usage {
            message: format!("More than one source account has the label {name}; use its ID."),
            hint: Some(Hint::run("anima source list")),
        }),
    }
}
