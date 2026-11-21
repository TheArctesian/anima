//! `anima agents sessions`: the single sessions of the AI agent programs, the newest use first,
//! with their sums and estimate over a range of UTC days (AIS-025), and which were used in the
//! last 30 minutes (AIS-026).

use chrono::Utc;

use super::documents::{AgentSessions, agent_sessions};
use super::report::tokens;
use super::usage::day_range;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::{Cell, Table, Tone};

/// The sessions one answer lists when no limit is given.
pub const DEFAULT_FIRST: i64 = 50;

/// What a sessions query asks, as the command and the MCP tool give it.
#[derive(Debug, Default)]
pub struct SessionsAsk {
    /// A device's ID or its exact name.
    pub device: Option<String>,
    pub source: Option<agent_sessions::AgentSource>,
    pub repository: Option<String>,
    pub first_day: Option<String>,
    pub last_day: Option<String>,
    /// 1 to 200.
    pub first: i64,
}

/// The K1 `AgentSessions` answer to `ask`.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device name; the hub's refusal.
pub async fn fetch(
    hub: &Hub,
    ask: SessionsAsk,
) -> Result<agent_sessions::AgentSessionsAgentSessions, Failure> {
    let device_id = match ask.device {
        Some(device) => Some(resolve(hub, &device).await?),
        None => None,
    };
    let variables = agent_sessions::Variables {
        filter: Some(agent_sessions::AgentUsageFilter {
            device_id,
            source: ask.source,
            repository: ask.repository,
            first_day: ask.first_day,
            last_day: ask.last_day,
        }),
        first: ask.first,
    };
    Ok(hub.run::<AgentSessions>(variables).await?.agent_sessions)
}

#[derive(Debug, clap::Args)]
pub struct SessionsArgs {
    /// Only this device: its ID or its exact name.
    #[arg(long)]
    device: Option<String>,
    /// Only this program.
    #[arg(long, value_enum)]
    source: Option<SourceArg>,
    /// Only this repository, as host/owner/name (github.com/owner/name).
    #[arg(long)]
    repository: Option<String>,
    /// The first UTC day, YYYY-MM-DD; 29 days before --until when not given.
    #[arg(long, conflicts_with = "days")]
    since: Option<String>,
    /// The last UTC day, YYYY-MM-DD, included; today when not given.
    #[arg(long)]
    until: Option<String>,
    /// The number of days that end on --until (or today), 1 to 366.
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..=366))]
    days: Option<u16>,
    /// The most sessions to list, 1 to 200.
    #[arg(long, default_value_t = DEFAULT_FIRST, value_parser = clap::value_parser!(i64).range(1..=200))]
    limit: i64,
}

/// K1 `AgentSource`.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum SourceArg {
    ClaudeCode,
    Codex,
}

impl SessionsArgs {
    /// Prints a row for each session and the notes under the table or, with `--json`, the K1
    /// `AgentSessions` answer.
    ///
    /// # Errors
    /// [`Failure::Usage`] for a bad `--until` with `--days`; as [`fetch`].
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let (first_day, last_day) =
            day_range((self.since, self.until), self.days, Utc::now().date_naive())?;
        let ask = SessionsAsk {
            device: self.device,
            source: self.source.map(|source| match source {
                SourceArg::ClaudeCode => agent_sessions::AgentSource::CLAUDE_CODE,
                SourceArg::Codex => agent_sessions::AgentSource::CODEX,
            }),
            repository: self.repository,
            first_day,
            last_day,
            first: self.limit,
        };
        let answer = fetch(&hub, ask).await?;
        invocation.output.data(&answer, table)?;
        if !invocation.output.json() {
            for line in notes(&answer) {
                invocation.output.line(&line)?;
            }
        }
        Ok(())
    }
}

/// One row for each session, the newest use first; a recent one is marked (AIS-026).
#[must_use]
pub fn table(answer: &agent_sessions::AgentSessionsAgentSessions) -> Table {
    let mut table = Table::new(vec![
        "LAST USE",
        "DEVICE",
        "SOURCE",
        "REPOSITORY",
        "MODEL",
        "PROMPTS",
        "TOOL CALLS",
        "TOKENS",
        "EST. COST (USD)",
    ]);
    for session in &answer.sessions {
        let all = session.input_tokens
            + session.output_tokens
            + session.cache_read_tokens
            + session.cache_write_tokens;
        let last_use = if session.recent {
            Cell::toned(&format!("{} (recent)", session.last_use_at), Tone::Good)
        } else {
            session.last_use_at.clone().into()
        };
        table.row(vec![
            last_use,
            session.device.name.clone().into(),
            source_name(&session.source).into(),
            session
                .repository
                .clone()
                .unwrap_or_else(|| "-".to_owned())
                .into(),
            session
                .model
                .clone()
                .unwrap_or_else(|| "-".to_owned())
                .into(),
            format!("{:.0}", session.prompts).into(),
            format!("{:.0}", session.tool_calls).into(),
            tokens(all).into(),
            session.estimated_cost_usd.map_or_else(
                || Cell::toned("no price", Tone::Muted),
                |usd| format!("{usd:.2}").into(),
            ),
        ]);
    }
    table
}

/// The lines under the table: the days, the sessions left out, what "recent" means and what the
/// estimate is.
#[must_use]
pub fn notes(answer: &agent_sessions::AgentSessionsAgentSessions) -> Vec<String> {
    let mut lines = vec![format!(
        "Days {} to {} (UTC).",
        answer.first_day, answer.last_day
    )];
    if answer.sessions_left_out > 0 {
        lines.push(format!(
            "{} more sessions are not shown: give a larger --limit or fewer days.",
            answer.sessions_left_out
        ));
    }
    lines.push(RECENT_NOTE.to_owned());
    lines.push(ESTIMATE_NOTE.to_owned());
    lines
}

/// What the estimate is, and what it is not (AIS-021).
pub const ESTIMATE_NOTE: &str =
    "The cost is an estimate from public list prices; it is not the bill of a subscription.";

/// What "recent" says, and what it does not (AIS-026).
pub const RECENT_NOTE: &str = "Recent: used in the last 30 minutes. The daemon counts every 15 \
     minutes, so this does not say that a session runs now.";

const fn source_name(source: &agent_sessions::AgentSource) -> &'static str {
    match source {
        agent_sessions::AgentSource::CLAUDE_CODE => "Claude Code",
        agent_sessions::AgentSource::CODEX => "Codex",
        agent_sessions::AgentSource::Other(_) => "other",
    }
}
