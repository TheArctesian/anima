//! How a place looks to a person: a table of places and one place as a record.

use serde_json::Value;

use crate::output::Table;

fn text<'a>(place: &'a Value, field: &str) -> &'a str {
    place[field].as_str().unwrap_or("-")
}

fn centre(place: &Value) -> String {
    match (
        place["latitude"].as_f64(),
        place["longitude"].as_f64(),
        place["radiusM"].as_i64(),
    ) {
        (Some(latitude), Some(longitude), Some(radius)) => {
            format!("{latitude:.5}, {longitude:.5} (within {radius} m)")
        }
        _ => "-".to_owned(),
    }
}

fn names(place: &Value, field: &str) -> String {
    let all: Vec<&str> = place[field]
        .as_array()
        .map_or_default(|list| list.iter().filter_map(Value::as_str).collect());
    if all.is_empty() {
        "-".to_owned()
    } else {
        all.join(", ")
    }
}

/// The places, one row each.
pub fn rows(places: &Value) -> Table {
    let mut table = Table::new(vec!["NAME", "CATEGORY", "PARENT", "CENTRE", "SITES", "ID"]);
    for place in places.as_array().map_or_default(Vec::as_slice) {
        let sites = place["siteIds"].as_array().map_or(0, Vec::len);
        table.row(vec![
            text(place, "name").into(),
            text(place, "category").into(),
            text(place, "parentId").into(),
            centre(place).into(),
            sites.to_string().into(),
            text(place, "id").into(),
        ]);
    }
    table
}

/// One place with each of its fields.
pub fn record(place: &Value) -> Table {
    let mut table = Table::record();
    table.pair("ID", text(place, "id"));
    table.pair("Name", text(place, "name"));
    table.pair("Category", text(place, "category"));
    table.pair("Icon", text(place, "icon"));
    table.pair("Parent", text(place, "parentId"));
    table.pair("Centre", centre(place));
    table.pair("Other names", names(place, "aliases"));
    table.pair("Geocoder name", text(place, "geocoderName"));
    table.pair("Sites", names(place, "siteIds"));
    table
}
