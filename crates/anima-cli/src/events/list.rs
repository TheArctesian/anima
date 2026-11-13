//! `anima event list`: the hub's events, newest first, with the K1 `EventFilter` and pages
//! (CMD-026).

use super::documents::{EventList, event_list};
use crate::connection_page;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only events with this severity; repeat for more than one.
    #[arg(long, value_enum)]
    severity: Vec<SeverityArg>,
    /// Only events from this source; repeat for more than one.
    #[arg(long, value_enum)]
    source: Vec<SourceArg>,
    /// Only events about this device, by name or ID.
    #[arg(long)]
    device: Option<String>,
    /// Only events at or after this time (RFC 3339, such as 2026-11-01T00:00:00Z).
    #[arg(long)]
    since: Option<String>,
    /// Events per page, 1 to 200.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum SeverityArg {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum SourceArg {
    Check,
    Session,
    Device,
    Access,
    Hub,
    Metrics,
    Notification,
    Vpn,
}

impl ListArgs {
    /// `anima event list` with the filters in use, for the next-page hint.
    fn next_page_command(&self) -> CommandLine {
        CommandLine::new("anima event list")
            .choices("--severity", &self.severity)
            .choices("--source", &self.source)
            .flag("--device", self.device.as_deref())
            .flag("--since", self.since.as_deref())
    }

    /// Prints the events as a table or, with `--json`, as the K1 `EventConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown device or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = self.next_page_command();
        let hub = invocation.hub()?;
        let device_id = match &self.device {
            Some(device) => Some(resolve(&hub, device).await?),
            None => None,
        };
        let filter = event_list::EventFilter {
            severity: Some(self.severity.iter().map(|s| s.list()).collect())
                .filter(|severities: &Vec<_>| !severities.is_empty()),
            source: Some(self.source.iter().map(|s| s.k1()).collect())
                .filter(|sources: &Vec<_>| !sources.is_empty()),
            device_id,
            since: self.since,
        };
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, filter) = (&hub, &filter);
        let events = fetch(self.after, self.all, move |after| async move {
            let variables = event_list::Variables {
                filter: Some(filter.clone()),
                first,
                after,
            };
            Ok(hub.run::<EventList>(variables).await?.events)
        })
        .await?;
        invocation.output.data(&events, table)?;
        note_next(invocation, &events, "events", &command);
        Ok(())
    }
}

connection_page!(event_list::EventListEvents);

fn table(events: &event_list::EventListEvents) -> Table {
    let mut table = Table::new(vec!["AT", "SEVERITY", "SOURCE", "DEVICE", "MESSAGE"]);
    for edge in &events.edges {
        let event = &edge.node;
        table.row(vec![
            event.at.as_str().into(),
            severity_cell(&enum_name(&event.severity)),
            enum_name(&event.source).into(),
            event.device_name.as_deref().unwrap_or("-").into(),
            event.message.as_str().into(),
        ]);
    }
    table
}

/// A K1 `Severity` in its colour.
fn severity_cell(name: &str) -> Cell {
    let tone = match name {
        "ERROR" => Tone::Bad,
        "WARNING" => Tone::Warning,
        _ => Tone::Muted,
    };
    Cell::toned(name, tone)
}

impl SeverityArg {
    const fn list(self) -> event_list::Severity {
        match self {
            Self::Info => event_list::Severity::INFO,
            Self::Warning => event_list::Severity::WARNING,
            Self::Error => event_list::Severity::ERROR,
        }
    }
}

impl SourceArg {
    const fn k1(self) -> event_list::EventSource {
        match self {
            Self::Check => event_list::EventSource::CHECK,
            Self::Session => event_list::EventSource::SESSION,
            Self::Device => event_list::EventSource::DEVICE,
            Self::Access => event_list::EventSource::ACCESS,
            Self::Hub => event_list::EventSource::HUB,
            Self::Metrics => event_list::EventSource::METRICS,
            Self::Notification => event_list::EventSource::NOTIFICATION,
            Self::Vpn => event_list::EventSource::VPN,
        }
    }
}
