//! What a command that changes a check prints: its name, ID, kind and state.

use crate::output::{Cell, Table};

/// `PAUSED` for a paused check, else the K1 status name.
#[must_use]
pub fn state(status: &str, paused: bool) -> String {
    if paused {
        "PAUSED".to_owned()
    } else {
        status.to_owned()
    }
}

/// The check as label and value lines.
#[must_use]
pub fn written(name: &str, id: &str, kind: &str, state: &str) -> Table {
    let mut table = Table::record();
    table.pair("Name", name);
    table.pair("ID", id);
    table.pair("Kind", kind);
    table.pair("Status", Cell::status(state));
    table
}
