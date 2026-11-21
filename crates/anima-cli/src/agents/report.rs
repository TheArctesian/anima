//! The usage table for a person: one row for each group and the total, tokens in readable
//! units, the cost marked as an estimate; and the notes below it (AIS-021, AIS-024).

use super::documents::agent_usage::{
    AgentSource, AgentUsageAgentUsage, AgentUsageAgentUsageAccounts, AgentUsageDimension,
    AgentUsageState, UsageSums,
};
use crate::output::{Cell, Table, Tone};

/// The sentence for a device whose daemon runs as the systemd system unit (AIS-024).
pub const SYSTEM_UNIT: &str =
    "This daemon runs as a system unit and cannot see the sessions of a user.";

/// The sentence for a device where the operator turned the counts off.
pub const TURNED_OFF: &str = "The use counts are turned off on this device.";

/// What the estimate is, and what it is not (AIS-021).
#[must_use]
pub fn estimate_note(prices_as_of: &str) -> String {
    format!(
        "The cost is an estimate from the public list prices of {prices_as_of}; it is not the \
         bill of a subscription."
    )
}

/// A key column of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Day,
    Device,
    Source,
    Model,
    Repository,
}

impl Column {
    #[must_use]
    pub const fn of(dimension: &AgentUsageDimension) -> Self {
        match dimension {
            AgentUsageDimension::DEVICE | AgentUsageDimension::Other(_) => Self::Device,
            AgentUsageDimension::SOURCE => Self::Source,
            AgentUsageDimension::MODEL => Self::Model,
            AgentUsageDimension::REPOSITORY => Self::Repository,
            AgentUsageDimension::DAY => Self::Day,
        }
    }

    const fn header(self) -> &'static str {
        match self {
            Self::Day => "DAY",
            Self::Device => "DEVICE",
            Self::Source => "SOURCE",
            Self::Model => "MODEL",
            Self::Repository => "REPOSITORY",
        }
    }

    fn cell(self, row: &UsageSums) -> String {
        let text = match self {
            Self::Day => row.day.clone(),
            Self::Device => row.device.as_ref().map(|device| device.name.clone()),
            Self::Source => row
                .source
                .as_ref()
                .map(|source| source_name(source).to_owned()),
            Self::Model => row.model.clone(),
            Self::Repository => row.repository.clone(),
        };
        text.unwrap_or_else(|| "-".to_owned())
    }
}

const COUNTS: [&str; 8] = [
    "SESSIONS",
    "PROMPTS",
    "TOOL CALLS",
    "INPUT",
    "OUTPUT",
    "CACHE READ",
    "CACHE WRITE",
    "EST. COST (USD)",
];

/// The rows of `answer` under the `columns` asked for, in the hub's order, then the total.
#[must_use]
pub fn table(answer: &AgentUsageAgentUsage, columns: &[Column]) -> Table {
    let mut header: Vec<&'static str> = columns.iter().map(|column| column.header()).collect();
    header.extend(COUNTS);
    let mut table = Table::new(header);
    for row in &answer.rows {
        let mut cells: Vec<Cell> = columns
            .iter()
            .map(|column| column.cell(row).into())
            .collect();
        cells.extend(sums(row));
        table.row(cells);
    }
    let mut total: Vec<Cell> = columns
        .iter()
        .enumerate()
        .map(|(at, _)| {
            if at == 0 {
                Cell::toned("total", Tone::Muted)
            } else {
                "".into()
            }
        })
        .collect();
    total.extend(sums(&answer.total));
    table.row(total);
    table
}

fn sums(row: &UsageSums) -> Vec<Cell> {
    let cost = row.estimated_cost_usd.map_or_else(
        || Cell::toned("no price", Tone::Muted),
        |usd| dollars(usd).into(),
    );
    vec![
        row.sessions.to_string().into(),
        whole(row.prompts).into(),
        whole(row.tool_calls).into(),
        tokens(row.input_tokens).into(),
        tokens(row.output_tokens).into(),
        tokens(row.cache_read_tokens).into(),
        tokens(row.cache_write_tokens).into(),
        cost,
    ]
}

/// The lines under the table: the days, the rows left out, what the estimate is, the tokens
/// it leaves out, and each device whose daemon cannot count.
#[must_use]
pub fn notes(answer: &AgentUsageAgentUsage) -> Vec<String> {
    let mut lines = vec![format!(
        "Days {} to {} (UTC).",
        answer.first_day, answer.last_day
    )];
    if answer.rows_left_out > 0 {
        lines.push(format!(
            "{} more rows are not shown: ask for fewer days or sum by fewer dimensions.",
            answer.rows_left_out
        ));
    }
    lines.push(estimate_note(&answer.prices_as_of));
    if answer.total.unpriced_tokens > 0.0 {
        lines.push(format!(
            "{} tokens of models with no known price are not in the estimate.",
            tokens(answer.total.unpriced_tokens)
        ));
    }
    for entry in &answer.devices {
        if let Some(sentence) = state_sentence(&entry.state) {
            lines.push(format!("{}: {sentence}", entry.device.name));
        }
    }
    lines.extend(answer.accounts.iter().map(account_line));
    lines
}

/// One account and its devices, by its short key: the limit is shared by its devices
/// (R-NEW-accountgroup). `Account 3f9a0c55 (Codex, Pro) on nas, studio: five_hour 35 % used,
/// resets 2026-11-21T14:00:00Z.`
#[must_use]
pub fn account_line(account: &AgentUsageAgentUsageAccounts) -> String {
    let plan = account
        .plan
        .as_deref()
        .map_or_else(String::new, |plan| format!(", {plan}"));
    let devices: Vec<&str> = account
        .devices
        .iter()
        .map(|one| one.name.as_str())
        .collect();
    let limits: Vec<String> = account
        .limits
        .iter()
        .map(|limit| {
            let used = match (limit.reached, limit.used_percent) {
                (true, _) => "reached".to_owned(),
                (false, Some(used)) => format!("{used} % used"),
                (false, None) => "not reached".to_owned(),
            };
            let resets = limit
                .resets_at
                .as_deref()
                .map_or_else(String::new, |at| format!(", resets {at}"));
            format!("{} {used}{resets}", limit.window)
        })
        .collect();
    let limits = if limits.is_empty() {
        "no limit reported".to_owned()
    } else {
        limits.join("; ")
    };
    format!(
        "Account {} ({}{plan}) on {}: {limits}.",
        account.short_key,
        source_name(&account.source),
        devices.join(", ")
    )
}

/// The sentence for a daemon that sends no counts; `None` for one that reads.
#[must_use]
pub const fn state_sentence(state: &AgentUsageState) -> Option<&'static str> {
    match state {
        AgentUsageState::SYSTEM_UNIT => Some(SYSTEM_UNIT),
        AgentUsageState::TURNED_OFF => Some(TURNED_OFF),
        AgentUsageState::READING | AgentUsageState::Other(_) => None,
    }
}

const fn source_name(source: &AgentSource) -> &'static str {
    match source {
        AgentSource::CLAUDE_CODE => "Claude Code",
        AgentSource::CODEX => "Codex",
        AgentSource::Other(_) => "other",
    }
}

/// A count as a whole number.
fn whole(count: f64) -> String {
    format!("{count:.0}")
}

/// A token count in readable units: `950`, `18.3k`, `2.2M`, `1.4B`.
#[must_use]
pub fn tokens(count: f64) -> String {
    for (size, unit) in [(1e9, "B"), (1e6, "M"), (1e3, "k")] {
        if count >= size {
            return format!("{:.1}{unit}", count / size);
        }
    }
    whole(count)
}

/// US dollars with cents; `<0.01` for a cost above zero that rounds to none.
fn dollars(usd: f64) -> String {
    if usd > 0.0 && usd < 0.005 {
        "<0.01".to_owned()
    } else {
        format!("{usd:.2}")
    }
}
