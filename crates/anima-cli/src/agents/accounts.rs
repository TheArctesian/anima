//! `anima agents accounts`: the account, the plan and the limits that each agent program of
//! this computer said to the runner (THR-132). It reads the local socket of this user only; the
//! name of an account is often an address of the person, so no MCP tool gives it (THR-132).

use agent_threads::control::{AccountRow, Action, Outcome};
use agent_threads::drive::Limit;
use chrono::{DateTime, Utc};

use super::drive::{accepted, program_key};
use super::runner::RunnerLink;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::{Cell, Table, Tone};

/// `anima agents accounts`.
///
/// # Errors
/// [`Failure`] when the runner is not there.
pub async fn accounts(invocation: &Invocation) -> Result<(), Failure> {
    let mut link = RunnerLink::connect().await?;
    let Outcome::Accounts { accounts } = accepted(link.call(Action::Accounts).await?)? else {
        return Ok(());
    };
    if accounts.is_empty() {
        invocation
            .output
            .message("No program of this computer said its account yet. Start a thread first.");
        return Ok(());
    }
    invocation.output.data(&accounts, |rows| table(rows))
}

fn table(rows: &[AccountRow]) -> Table {
    let mut table = Table::new(vec!["PROGRAM", "ACCOUNT", "PLAN", "LIMITS", "REPORTED"]);
    for row in rows {
        let name = match &row.facts.name {
            _ if row.facts.signed_out => "Signed out",
            Some(name) => name.reveal(),
            None => "Account not known",
        };
        table.row(vec![
            Cell::new(super::threads::shown::program_words(program_key(
                row.program,
            ))),
            Cell::new(name),
            Cell::new(row.facts.plan.as_deref().unwrap_or("-")),
            Cell::new(&limits_text(&row.facts.limits)),
            Cell::toned(&row.at.format("%H:%M").to_string(), Tone::Muted),
        ]);
    }
    table
}

/// Each limit as `window used% (resets HH:MM)`, the limits joined by `; `.
fn limits_text(limits: &[Limit]) -> String {
    if limits.is_empty() {
        return "-".to_owned();
    }
    limits
        .iter()
        .map(|limit| {
            let reset = limit
                .resets_at
                .and_then(|at| DateTime::<Utc>::from_timestamp(at, 0))
                .map_or_else(String::new, |at| {
                    format!(
                        " (resets {})",
                        at.with_timezone(&chrono::Local).format("%a %H:%M")
                    )
                });
            let reached = if limit.reached { ", reached" } else { "" };
            let used = limit
                .used_percent
                .map_or_else(String::new, |used| format!(" {used}%"));
            format!("{}{used}{reached}{reset}", limit.window)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_132_limits_read_as_words() {
        let limit = |window: &str, used: Option<u8>, reached| Limit {
            window: window.into(),
            used_percent: used,
            resets_at: None,
            reached,
        };
        assert_eq!(limits_text(&[]), "-");
        assert_eq!(
            limits_text(&[
                limit("300 min", Some(42), false),
                limit("seven_day", Some(100), true),
                limit("five_hour", None, true)
            ]),
            "300 min 42%; seven_day 100%, reached; five_hour, reached"
        );
    }
}
