//! `anima location context <event>`: the roles of one file transfer and the evidence for
//! where the operator was (K-E4 Section 12). The place of a device is configured; the place of
//! the operator comes only from a sample at the time.

use serde_json::Value;

use super::documents::{LocationContext, location_context};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct ContextArgs {
    /// The ID of the transfer event (see `anima event list`).
    event: String,
}

impl ContextArgs {
    /// Prints the context as a record or, with `--json`, as the K1 `TransferContext`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when the event is not a transfer; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = location_context::Variables { event: self.event };
        let answer = hub.run::<LocationContext>(variables).await?;
        let Some(context) = answer.transfer_context else {
            return Err(Failure::NotFound {
                message: "This event is not a file transfer, so it has no context.".to_owned(),
                hint: None,
            });
        };
        invocation.output.data(&context, |context| {
            record(&serde_json::to_value(context).unwrap_or_default())
        })
    }
}

fn known(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| "unknown".to_owned(), str::to_owned)
}

fn end(table: &mut Table, role: &str, end: &Value) {
    for (label, field) in [
        ("device", "deviceId"),
        ("site", "siteId"),
        ("place", "placeId"),
    ] {
        table.row(vec![
            format!("{role} {label}").as_str().into(),
            known(&end[field]).as_str().into(),
        ]);
    }
}

/// The words for the evidence of the operator's position: an inferred or uncertain sample is
/// never shown as a confirmed place.
#[must_use]
pub fn evidence(operator: &Value) -> String {
    let offset = operator["offsetSeconds"].as_i64().unwrap_or_default();
    let uncertain = if operator["uncertain"].as_bool().unwrap_or_default() {
        ", uncertain"
    } else {
        ""
    };
    match operator["basis"].as_str().unwrap_or_default() {
        "AT_THE_TIME" => format!("a sample at the time ({offset:+} s{uncertain})"),
        "NEAREST_INFERRED" => format!("inferred from the nearest sample ({offset:+} s{uncertain})"),
        _ => "unknown: no sample within 10 minutes".to_owned(),
    }
}

fn record(context: &Value) -> Table {
    let mut table = Table::new(vec!["FIELD", "VALUE"]);
    let text = |field: &str| known(&context[field]);
    table.row(vec!["event".into(), text("eventId").as_str().into()]);
    table.row(vec!["time".into(), text("occurredAt").as_str().into()]);
    table.row(vec!["direction".into(), text("direction").as_str().into()]);
    end(&mut table, "source", &context["source"]);
    end(&mut table, "destination", &context["destination"]);
    let actor = format!("{} ({})", text("actor"), text("actorKind"));
    table.row(vec!["started by".into(), actor.as_str().into()]);
    table.row(vec![
        "client device".into(),
        text("clientDeviceId").as_str().into(),
    ]);
    let operator = &context["operator"];
    table.row(vec![
        "operator evidence".into(),
        evidence(operator).as_str().into(),
    ]);
    table.row(vec![
        "operator place".into(),
        known(&operator["placeId"]).as_str().into(),
    ]);
    table
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::evidence;

    #[test]
    fn r_145_the_words_keep_inferred_uncertain_and_unknown_apart() {
        let at = json!({ "basis": "AT_THE_TIME", "offsetSeconds": -60, "uncertain": false });
        assert_eq!(evidence(&at), "a sample at the time (-60 s)");
        let near = json!({ "basis": "NEAREST_INFERRED", "offsetSeconds": 300, "uncertain": true });
        assert_eq!(
            evidence(&near),
            "inferred from the nearest sample (+300 s, uncertain)"
        );
        let none = json!({ "basis": "UNKNOWN", "uncertain": false });
        assert_eq!(evidence(&none), "unknown: no sample within 10 minutes");
    }
}
