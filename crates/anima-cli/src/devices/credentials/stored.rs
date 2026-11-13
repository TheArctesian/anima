//! What `credential set` and `clear` print: the access setting and whether a credential is kept.

use crate::output::{Cell, Table, Tone};

/// One row: `protocol`, whether the access is on, its port and whether a credential is kept.
#[must_use]
pub fn table(protocol: &str, enabled: bool, port: i64, credential_stored: bool) -> Table {
    let mut table = Table::record();
    table.pair("Access", protocol);
    table.pair("Enabled", if enabled { "yes" } else { "no" });
    table.pair("Port", port.to_string());
    let (text, tone) = if credential_stored {
        ("stored", Tone::Good)
    } else {
        ("none", Tone::Muted)
    };
    table.pair("Credential", Cell::toned(text, tone));
    table
}
