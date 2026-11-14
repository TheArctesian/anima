//! How a directory reads in a table.

use serde::Serialize;

use crate::forge::render::{many, number, text};
use crate::output::Table;

/// The entries of a K1 `CodeDirectory` as rows: kind, size, name.
#[must_use]
pub fn directory<T: Serialize>(directory: &T) -> Table {
    let directory = serde_json::to_value(directory).unwrap_or_default();
    let mut table = Table::new(vec!["KIND", "SIZE", "NAME"]);
    for entry in many(&directory, "entries") {
        let size = match text(entry, "kind") {
            "FILE" => number(entry, "size"),
            _ => "-".to_owned(),
        };
        table.row(vec![
            text(entry, "kind").to_lowercase().into(),
            size.into(),
            text(entry, "name").into(),
        ]);
    }
    table
}
