//! The agent usage GraphQL part: `Query.agentUsage` (K1; R-135; AIS-020, AIS-021, AIS-024).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::sync::Arc;

use async_graphql::{Context, Enum, ID, InputObject, Object, Result, SimpleObject};
use chrono::NaiveDate;
use daemon_core::agent_usage::{Source, UsageState, is_repository};

use super::{
    Ask, Dimension, DimensionSet, Error, default_days,
    grouping::{self, Placed},
    model::Group,
    prices::PRICES_AS_OF,
};
use crate::{
    api_errors::{FieldError, internal, invalid, pool, schema_dependency},
    clock::Clock,
    fleet::{
        access::Authenticated,
        devices::{self, Device},
    },
};

/// The most rows one answer lists; the rest are counted in `rowsLeftOut`.
const MAX_ROWS: usize = 1000;

/// The most days one query may span.
const MAX_DAYS: i64 = 366;

/// `AgentSource`: the AI agent program a session comes from (AIS-018).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum AgentSource {
    ClaudeCode,
    Codex,
}

impl From<Source> for AgentSource {
    fn from(source: Source) -> Self {
        match source {
            Source::ClaudeCode => Self::ClaudeCode,
            Source::Codex => Self::Codex,
        }
    }
}

impl From<AgentSource> for Source {
    fn from(source: AgentSource) -> Self {
        match source {
            AgentSource::ClaudeCode => Self::ClaudeCode,
            AgentSource::Codex => Self::Codex,
        }
    }
}

/// `AgentUsageState`: what a device's daemon can read of the sessions (AIS-024).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum AgentUsageState {
    Reading,
    SystemUnit,
    TurnedOff,
}

impl From<UsageState> for AgentUsageState {
    fn from(state: UsageState) -> Self {
        match state {
            UsageState::Reading => Self::Reading,
            UsageState::SystemUnit => Self::SystemUnit,
            UsageState::TurnedOff => Self::TurnedOff,
        }
    }
}

/// K1 `AgentUsageFilter`.
#[derive(Debug, Default, InputObject)]
pub struct AgentUsageFilter {
    device_id: Option<ID>,
    source: Option<AgentSource>,
    repository: Option<String>,
    first_day: Option<String>,
    last_day: Option<String>,
}

/// K1 `AgentUsageRow`: the sums of one group.
#[derive(SimpleObject)]
pub struct AgentUsageRow {
    device: Option<Device>,
    source: Option<AgentSource>,
    model: Option<String>,
    repository: Option<String>,
    day: Option<String>,
    sessions: i64,
    prompts: f64,
    tool_calls: f64,
    input_tokens: f64,
    output_tokens: f64,
    cache_read_tokens: f64,
    cache_write_tokens: f64,
    estimated_cost_usd: Option<f64>,
    unpriced_tokens: f64,
}

/// K1 `AgentUsageDevice`.
#[derive(SimpleObject)]
pub struct AgentUsageDevice {
    device: Device,
    state: AgentUsageState,
}

/// K1 `AgentUsage`.
#[derive(SimpleObject)]
#[graphql(name = "AgentUsage")]
pub struct AgentUsageAnswer {
    first_day: String,
    last_day: String,
    rows: Vec<AgentUsageRow>,
    rows_left_out: i32,
    total: AgentUsageRow,
    devices: Vec<AgentUsageDevice>,
    prices_as_of: String,
}

impl AgentUsageRow {
    fn of(device: Option<Device>, group: Group) -> Self {
        let tokens = group.sums.tokens;
        Self {
            device,
            source: group.key.source.map(AgentSource::from),
            model: group.key.model,
            repository: group.key.repository,
            day: group.key.day.map(|day| day.to_string()),
            sessions: group.sums.sessions,
            prompts: group.sums.prompts,
            tool_calls: group.sums.tool_calls,
            input_tokens: tokens.input,
            output_tokens: tokens.output,
            cache_read_tokens: tokens.cache_read,
            cache_write_tokens: tokens.cache_write,
            estimated_cost_usd: group.estimated_cost_usd,
            unpriced_tokens: group.unpriced_tokens,
        }
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "AgentUsageQuery")]
impl Query {
    /// The use of the AI agent programs, summed by the dimensions of `by` over a range of UTC
    /// days (AIS-020), with the estimate of its cost (AIS-021) and each daemon's state
    /// (AIS-024).
    #[graphql(guard = "Authenticated")]
    async fn agent_usage(
        &self,
        ctx: &Context<'_>,
        filter: Option<AgentUsageFilter>,
        #[graphql(default)] by: Vec<Dimension>,
    ) -> Result<AgentUsageAnswer> {
        let today = schema_dependency::<Arc<dyn Clock>>(ctx)?.now().date_naive();
        let ask = ask(filter.unwrap_or_default(), &by, today).map_err(|bad| invalid(&bad))?;
        let db = pool(ctx)?;
        let report = super::usage(db, &ask).await.map_err(usage_error)?;
        let devices = devices::list(db).await.map_err(internal)?;
        let placed = grouping::arranged(report.groups, &devices);
        let left_out = placed.len().saturating_sub(MAX_ROWS);
        let rows = placed
            .into_iter()
            .take(MAX_ROWS)
            .map(|Placed { device, group }| AgentUsageRow::of(device, group))
            .collect();
        let states = grouping::states_with_devices(&report.states, &devices);
        Ok(AgentUsageAnswer {
            first_day: ask.first_day.to_string(),
            last_day: ask.last_day.to_string(),
            rows,
            rows_left_out: i32::try_from(left_out).unwrap_or(i32::MAX),
            total: AgentUsageRow::of(None, report.total),
            devices: states
                .into_iter()
                .map(|(device, state)| AgentUsageDevice {
                    device,
                    state: state.into(),
                })
                .collect(),
            prices_as_of: PRICES_AS_OF.to_owned(),
        })
    }
}

/// `VALIDATION` for too many groups; anything else is internal.
fn usage_error(error: Error) -> async_graphql::Error {
    match error {
        Error::TooManyGroups => invalid(&[FieldError::new(
            "by",
            "Too many groups: group by fewer dimensions or ask for fewer days.",
        )]),
        other => internal(other),
    }
}

/// The ask the arguments make on `today` (UTC), or each argument the operator must fix.
fn ask(
    filter: AgentUsageFilter,
    by: &[Dimension],
    today: NaiveDate,
) -> Result<Ask, Vec<FieldError>> {
    let mut bad = Vec::new();
    let device = filter.device_id.and_then(|id| {
        id.parse()
            .map_err(|_| bad.push(FieldError::new("deviceId", "That id is not valid.")))
            .ok()
    });
    if filter
        .repository
        .as_deref()
        .is_some_and(|text| !is_repository(text))
    {
        bad.push(FieldError::new(
            "repository",
            "Give a repository as host/owner/name, for example github.com/owner/name.",
        ));
    }
    let first = day_of(filter.first_day.as_deref(), "firstDay", &mut bad);
    let last = day_of(filter.last_day.as_deref(), "lastDay", &mut bad);
    let last_day = last.unwrap_or(today);
    let first_day = first.unwrap_or_else(|| default_days(last_day).0);
    let span = (last_day - first_day).num_days() + 1;
    if span < 1 {
        bad.push(FieldError::new(
            "firstDay",
            "The first day must not be after the last day.",
        ));
    } else if span > MAX_DAYS {
        bad.push(FieldError::new("firstDay", "Ask for at most 366 days."));
    }
    if !bad.is_empty() {
        return Err(bad);
    }
    Ok(Ask {
        device,
        source: filter.source.map(Source::from),
        repository: filter.repository,
        first_day,
        last_day,
        by: DimensionSet::of(by),
    })
}

/// The day `text` names, `YYYY-MM-DD`; `None` when there is none, with a field error when it
/// is not a real day so written.
fn day_of(text: Option<&str>, field: &'static str, bad: &mut Vec<FieldError>) -> Option<NaiveDate> {
    let text = text?;
    let day = (text.len() == 10)
        .then(|| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
        .flatten();
    if day.is_none() {
        bad.push(FieldError::new(field, "Give a day as YYYY-MM-DD."));
    }
    day
}
