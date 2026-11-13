//! A site as `list`, `create` and `set-wake-relay` print it.

use crate::output::Table;

/// The fields every site document selects, as text.
pub struct Shown<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub subnet: &'a str,
    pub wake_relay: Option<&'a str>,
}

impl Shown<'_> {
    #[must_use]
    pub fn table(&self) -> Table {
        let mut table = Table::record();
        table.pair("Name", self.name);
        table.pair("ID", self.id);
        table.pair("Subnet", self.subnet);
        table.pair("Wake relay", self.wake_relay.unwrap_or("-"));
        table
    }
}
