//! How a rule and a firing look to a person.

use serde_json::Value;

use crate::output::Table;

fn text(value: &Value, field: &str) -> String {
    match &value[field] {
        Value::String(text) => text.clone(),
        Value::Null => "-".to_owned(),
        other => other.to_string(),
    }
}

/// The trigger in a few words: its type and the fields that say when.
#[must_use]
pub fn trigger_words(trigger: &Value) -> String {
    let field = |name: &str| trigger[name].as_str().unwrap_or_default().to_owned();
    match trigger["type"].as_str().unwrap_or_default() {
        "time" if trigger["once"].is_string() => format!("once at {}", field("once")),
        "time" => format!("at {} ({})", field("at"), field("zone")),
        "place" => format!("{} {}", field("on"), field("place")),
        "source_event" => format!("{} {}", field("source"), field("kind")),
        "source_state" => format!("source {}", field("state")),
        "threshold" => format!("{} {} {}", field("key"), field("is"), trigger["value"]),
        "expiry" => format!(
            "{} ends in {} days",
            field("subject"),
            trigger["days_before"]
        ),
        other => other.to_owned(),
    }
}

/// The rules, one row each.
pub fn rules(rules: &Value) -> Table {
    let mut table = Table::new(vec!["NAME", "TRIGGER", "ON", "TODAY", "LAST", "ID"]);
    for rule in rules.as_array().map_or_default(Vec::as_slice) {
        let on = if rule["enabled"].as_bool().unwrap_or_default() {
            "yes"
        } else {
            "no"
        };
        table.row(vec![
            text(rule, "name").as_str().into(),
            trigger_words(&rule["trigger"]).as_str().into(),
            on.into(),
            text(rule, "firedToday").as_str().into(),
            text(rule, "lastFiringAt").as_str().into(),
            text(rule, "id").as_str().into(),
        ]);
    }
    table
}

/// One rule as a record.
pub fn rule(rule: &Value) -> Table {
    let mut table = Table::new(vec!["FIELD", "VALUE"]);
    let fields = [
        ("id", "id"),
        ("name", "name"),
        ("message", "message"),
        ("trigger", "trigger"),
        ("unless", "unless"),
        ("cooldown minutes", "cooldownMinutes"),
        ("daily limit", "dailyLimit"),
        ("channels", "channelIds"),
        ("on", "enabled"),
        ("revision", "revision"),
        ("fired today", "firedToday"),
        ("last firing", "lastFiringAt"),
    ];
    for (label, field) in fields {
        table.row(vec![label.into(), text(rule, field).as_str().into()]);
    }
    table
}

/// The firings, one row each.
pub fn firings(firings: &Value) -> Table {
    let mut table = Table::new(vec!["DUE", "STATE", "REASON", "TEXT", "ID"]);
    for firing in firings.as_array().map_or_default(Vec::as_slice) {
        table.row(vec![
            text(firing, "dueAt").as_str().into(),
            text(firing, "state").as_str().into(),
            text(firing, "reason").as_str().into(),
            text(firing, "text").as_str().into(),
            text(firing, "id").as_str().into(),
        ]);
    }
    table
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::trigger_words;

    #[test]
    fn rem_001_a_trigger_is_shown_in_a_few_words() {
        let daily = json!({ "type": "time", "at": "09:00", "zone": "Europe/Amsterdam" });
        assert_eq!(trigger_words(&daily), "at 09:00 (Europe/Amsterdam)");
        let expiry = json!({ "type": "expiry", "subject": "documents.passport",
                             "days_before": [21, 7] });
        assert_eq!(
            trigger_words(&expiry),
            "documents.passport ends in [21,7] days"
        );
    }
}
