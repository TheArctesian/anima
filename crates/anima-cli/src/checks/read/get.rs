//! `anima check get <id-or-name>`: one check, its kind and settings. The only command that
//! shows a push check's path, which holds its token.

use super::documents::{CheckGet, check_get};
use crate::checks::resolve::CheckRef;
use crate::checks::settings::{label, settings};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    #[command(flatten)]
    check: CheckRef,
}

impl GetArgs {
    /// Prints the check as label and value lines or, with `--json`, as the K1 `Check`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when no check has the ID or name; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let answer = hub
            .run::<CheckGet>(check_get::Variables { id: id.clone() })
            .await?;
        let check = answer.check.ok_or_else(|| Failure::NotFound {
            message: format!("No check has the ID {id}."),
            hint: Some("anima check list --device <id-or-name>".to_owned()),
        })?;
        invocation.output.data(&check, table)
    }
}

fn table(check: &check_get::CheckGetCheck) -> Table {
    let mut table = Table::record();
    table.pair("Name", check.name.as_str());
    table.pair("ID", check.id.as_str());
    table.pair("Kind", enum_name(&check.type_));
    let status = if check.paused {
        "PAUSED".to_owned()
    } else {
        enum_name(&check.status)
    };
    table.pair("Status", Cell::status(&status));
    table.pair("Device", check.device.name.as_str());
    if let Some(parent) = &check.parent {
        table.pair("Group", parent.name.as_str());
    }
    let config = serde_json::to_value(&check.config).unwrap_or_default();
    for (name, value) in settings(&config) {
        table.pair(&label(&name), value);
    }
    table.pair("Every", format!("{} s", check.interval_seconds));
    table.pair(
        "Retries",
        format!(
            "{}, {} s apart",
            check.retries, check.retry_interval_seconds
        ),
    );
    table.pair(
        "Degraded above",
        format!("{} ms", check.degraded_threshold_ms),
    );
    table.pair("Inverted", if check.invert { "yes" } else { "no" });
    if let Some(days) = check.certificate_days_left {
        table.pair("Certificate days left", days.to_string());
    }
    if let Some(days) = check.domain_days_left {
        table.pair("Domain days left", days.to_string());
    }
    notification_rows(&mut table, &check.notifications);
    if let Some(beat) = &check.last_heartbeat {
        table.pair("Last heartbeat", heartbeat_line(beat));
    }
    table.pair(
        "Uptime 24 h",
        check
            .uptime
            .map_or_else(|| "-".to_owned(), |ratio| format!("{:.2} %", ratio * 100.0)),
    );
    table
}

fn notification_rows(table: &mut Table, notifications: &check_get::CheckGetCheckNotifications) {
    let channels: Vec<&str> = notifications
        .channels
        .iter()
        .map(|channel| channel.name.as_str())
        .collect();
    table.pair(
        "Notifies",
        if channels.is_empty() {
            "-".to_owned()
        } else {
            channels.join(", ")
        },
    );
    table.pair(
        "Resend every",
        match notifications.resend_every_beats {
            0 => "never".to_owned(),
            beats => format!("{beats} Down heartbeats"),
        },
    );
}

fn heartbeat_line(beat: &check_get::CheckGetCheckLastHeartbeat) -> String {
    let time = beat
        .response_time_ms
        .map(|ms| format!(" {ms} ms"))
        .unwrap_or_default();
    let message = beat
        .message
        .as_ref()
        .map(|text| format!(" {text}"))
        .unwrap_or_default();
    format!("{} {}{time}{message}", beat.at, enum_name(&beat.status))
}
