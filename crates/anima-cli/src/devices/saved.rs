//! A device as `create` and `update` print it after the hub saved it.

use crate::output::{Cell, Table};

/// The fields both documents select, as text.
pub struct Saved<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub host: &'a str,
    pub site: &'a str,
    pub status: &'a str,
}

impl Saved<'_> {
    #[must_use]
    pub fn table(&self) -> Table {
        let mut table = Table::record();
        table.pair("Name", self.name);
        table.pair("ID", self.id);
        table.pair("Site", self.site);
        table.pair("Host", self.host);
        table.pair("Status", Cell::status(self.status));
        table
    }
}
