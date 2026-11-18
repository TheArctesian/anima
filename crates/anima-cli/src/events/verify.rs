//! `anima audit verify`: checks the hash chain of the audit log on the hub (K1 `auditChain`,
//! LOG-027). It changes nothing.

use super::documents::{AuditChain, audit_chain};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct VerifyArgs;

impl VerifyArgs {
    /// Prints whether the chain is whole, the first fault, and the head to keep outside the
    /// hub; with `--json` the K1 `AuditChainCheck`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<AuditChain>(audit_chain::Variables).await?;
        invocation.output.data(&answer.audit_chain, table)
    }
}

fn table(check: &audit_chain::AuditChainAuditChain) -> Table {
    let mut table = Table::record();
    table.pair("Chain", if check.intact { "whole" } else { "broken" });
    if let Some(fault) = &check.fault {
        let place = fault
            .seq
            .map_or_else(|| "no link".to_owned(), |seq| format!("link {seq}"));
        table.pair("Fault", format!("{} at {place}", enum_name(&fault.kind)));
        table.pair("Entry", fault.entry_id.as_str());
        table.pair("Entry time", fault.at.as_str());
    }
    table.pair("Links", check.links.to_string());
    table.pair("Dropped by retention", check.dropped.to_string());
    if let (Some(seq), Some(hash)) = (check.head_seq, &check.head_hash) {
        table.pair("Head", format!("{seq} {hash}"));
    }
    table
}
