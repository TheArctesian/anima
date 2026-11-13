//! A web interface as the documents of this noun print it.

use crate::output::{Cell, Table};

/// The fields every web interface document selects, as text.
pub struct Shown<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub url: &'a str,
    pub open_in: &'a str,
    pub pinned_fingerprint: Option<&'a str>,
}

impl Shown<'_> {
    /// Label and value lines for one web interface.
    #[must_use]
    pub fn table(&self) -> Table {
        let mut table = Table::record();
        table.pair("Label", self.label);
        table.pair("ID", self.id);
        table.pair("URL", self.url);
        table.pair("Opens in", self.open_in);
        table.pair("Pinned certificate", self.pinned_fingerprint.unwrap_or("-"));
        table
    }

    /// One row of a list under `ID LABEL URL OPENS IN PINNED`.
    #[must_use]
    pub fn row(&self) -> Vec<Cell> {
        vec![
            self.id.into(),
            self.label.into(),
            self.url.into(),
            self.open_in.into(),
            self.pinned_fingerprint.unwrap_or("-").into(),
        ]
    }
}
