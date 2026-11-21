//! How a route reads in a terminal. Each document has its own generated type, so the table is
//! made from the JSON form that each of them serialises to.

use serde::Serialize;
use serde_json::Value;

use crate::output::{Cell, Table, Tone};

fn text(route: &Value, field: &str) -> String {
    match route.get(field) {
        Some(Value::String(text)) => text.to_lowercase().replace('_', " "),
        Some(Value::Null) | None => "-".to_owned(),
        Some(other) => other.to_string(),
    }
}

fn state(route: &Value) -> Cell {
    match route.get("waitsFor") {
        Some(Value::String(waits)) => Cell::toned(
            &format!("waits for {}", waits.to_lowercase().replace('_', " ")),
            Tone::Warning,
        ),
        _ => Cell::toned("serves", Tone::Good),
    }
}

/// A row for each route.
pub fn table<T: Serialize>(routes: &[T]) -> Table {
    let mut table = Table::new(vec!["ID", "NAME", "DEVICE", "PORT", "PROOF", "STATE"]);
    for route in routes {
        let route = serde_json::to_value(route).unwrap_or_default();
        table.row(vec![
            text(&route, "id").into(),
            text(&route, "name").into(),
            text(&route, "deviceId").into(),
            text(&route, "port").into(),
            text(&route, "proof").into(),
            state(&route),
        ]);
    }
    table
}

/// One route with its proof record, its certificate and its counts.
pub fn record<T: Serialize>(route: &T) -> Table {
    let route = serde_json::to_value(route).unwrap_or_default();
    let mut table = Table::record();
    for (label, field) in [
        ("ID", "id"),
        ("Name", "name"),
        ("Application", "applicationId"),
        ("Device", "deviceId"),
        ("Port", "port"),
        ("Proof", "proof"),
        ("Failed reads", "failedReads"),
        ("TXT record", "proofRecordName"),
        ("TXT value", "proofRecordValue"),
        ("Machine consent", "machineConsent"),
        ("Certificate until", "certificateNotAfter"),
        ("Requests", "requests"),
        ("Bytes in", "bytesIn"),
        ("Bytes out", "bytesOut"),
        ("Refused", "refused"),
    ] {
        table.pair(label, text(&route, field));
    }
    table.pair("State", state(&route));
    table
}
